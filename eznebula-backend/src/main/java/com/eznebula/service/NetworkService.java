package com.eznebula.service;

import com.eznebula.config.EzNebulaProperties;
import com.eznebula.domain.entity.ClientNode;
import com.eznebula.domain.entity.NetworkGroup;
import com.eznebula.domain.repository.ClientNodeRepository;
import com.eznebula.domain.repository.NetworkGroupRepository;
import com.eznebula.dto.request.JoinNetworkRequest;
import com.eznebula.dto.response.ClientInfo;
import com.eznebula.dto.response.JoinNetworkResponse;
import com.eznebula.exception.AuthenticationException;
import com.eznebula.exception.EzNebulaException;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.scheduling.annotation.Scheduled;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.time.LocalDateTime;
import java.util.Collections;
import java.util.List;
import java.util.stream.Collectors;

/**
 * Service for managing network operations.
 * Groups are auto-created on first join and auto-deleted when empty.
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class NetworkService {

    private final NetworkGroupRepository networkGroupRepository;
    private final ClientNodeRepository clientNodeRepository;
    private final NebulaCertService nebulaCertService;
    private final IpAllocationService ipAllocationService;
    private final EzNebulaProperties properties;

    /** Base CIDR for auto-created groups — each group gets a /24 within this /16 */
    private static final String BASE_CIDR = "10.168.0.0/16";

    /**
     * Handle client join request.
     * If the group doesn't exist, it is auto-created (open, no token).
     */
    @Transactional
    public JoinNetworkResponse joinNetwork(JoinNetworkRequest request) {
        log.info("Join request: group={}, client={}", request.getGroupName(), request.getClientName());

        // Step 1: Find or auto-create the group
        NetworkGroup group = resolveGroup(request);

        // Step 2: Reuse existing client if same public key reconnects
        String virtualIp;
        int cidrSuffix;

        var existingClient = clientNodeRepository.findByPublicKey(request.getClientPublicKey());
        if (existingClient.isPresent()) {
            ClientNode existing = existingClient.get();
            virtualIp = existing.getVirtualIp();
            cidrSuffix = existing.getCidrSuffix();
            log.info("Reusing existing client {} with IP {}/{}", existing.getClientName(), virtualIp, cidrSuffix);

            // Update name and mark active
            existing.setClientName(request.getClientName());
            existing.setLastSeenAt(LocalDateTime.now());
            existing.setActive(true);
            clientNodeRepository.save(existing);
        } else {
            // Allocate new virtual IP
            virtualIp = ipAllocationService.allocateIp(group);
            cidrSuffix = ipAllocationService.getCidrSuffix(group.getCidrBlock());
            log.info("Allocated IP {}/{} for {}", virtualIp, cidrSuffix, request.getClientName());
        }

        // Step 3: Sign certificate (re-sign every time for fresh certificate)
        String clientCertificate = nebulaCertService.signCertificate(
                request.getClientName(),
                request.getClientPublicKey(),
                virtualIp,
                cidrSuffix,
                Collections.singletonList(group.getGroupName())
        );

        // Step 4: Get CA certificate
        String caCertificate = nebulaCertService.getCACertificate();

        // Step 5: Save/update client node
        ClientNode node = existingClient.orElseGet(() -> ClientNode.builder()
                .networkGroup(group)
                .publicKey(request.getClientPublicKey())
                .build());
        node.setClientName(request.getClientName());
        node.setVirtualIp(virtualIp);
        node.setCidrSuffix(cidrSuffix);
        node.setCertificate(clientCertificate);
        node.setCertificateExpiresAt(LocalDateTime.now().plusYears(10));
        node.setLastSeenAt(LocalDateTime.now());
        node.setActive(true);

        clientNodeRepository.save(node);

        log.info("Client {} joined group {} with IP {}/{}",
                request.getClientName(), group.getGroupName(), virtualIp, cidrSuffix);

        return JoinNetworkResponse.builder()
                .virtualIpWithCidr(virtualIp + "/" + cidrSuffix)
                .clientCertificate(clientCertificate)
                .caCertificate(caCertificate)
                .lighthouseIp(properties.getLighthouse().getPublicIp())
                .lighthouseNebulaIp(properties.getLighthouse().getNebulaIp())
                .lighthousePort(properties.getLighthouse().getPort())
                .networkCidr(group.getCidrBlock())
                .message("Joined group: " + group.getGroupName())
                .build();
    }

    /**
     * Resolve group — returns existing group or auto-creates one.
     */
    private NetworkGroup resolveGroup(JoinNetworkRequest request) {
        return networkGroupRepository
                .findByGroupNameAndActiveTrue(request.getGroupName())
                .map(group -> {
                    // Group exists: validate token if set
                    if (group.getJoinToken() != null && !group.getJoinToken().isEmpty()) {
                        if (!constantTimeEquals(group.getJoinToken(), request.getJoinToken())) {
                            log.warn("Invalid token for group: {}", request.getGroupName());
                            throw new AuthenticationException("Invalid join token");
                        }
                    }
                    return group;
                })
                .orElseGet(() -> autoCreateGroup(request.getGroupName()));
    }

    /**
     * Auto-create a new network group with a unique /24 subnet.
     */
    private NetworkGroup autoCreateGroup(String groupName) {
        log.info("Auto-creating group: {}", groupName);

        List<NetworkGroup> allGroups = networkGroupRepository.findAll();
        String cidrBlock = ipAllocationService.allocateGroupCidr(allGroups, BASE_CIDR);
        long startingIp = ipAllocationService.calculateStartingIp(cidrBlock);

        NetworkGroup group = NetworkGroup.builder()
                .groupName(groupName)
                .joinToken(null) // open group — no token required
                .cidrBlock(cidrBlock)
                .nextIpAddress(startingIp)
                .description("Auto-created group")
                .active(true)
                .build();

        return networkGroupRepository.save(group);
    }

    /**
     * Periodic cleanup: mark stale clients inactive, delete empty groups.
     * Runs every 30 seconds.
     */
    @Scheduled(fixedRate = 30_000)
    @Transactional
    public void cleanupStaleClients() {
        List<NetworkGroup> allGroups = networkGroupRepository.findAll();
        LocalDateTime staleCutoff = LocalDateTime.now().minusSeconds(90);
        LocalDateTime deleteCutoff = LocalDateTime.now().minusHours(24);

        for (NetworkGroup group : allGroups) {
            // Mark clients inactive if no heartbeat for 90s
            List<ClientNode> activeClients = clientNodeRepository.findByNetworkGroupAndActiveTrue(group);
            for (ClientNode client : activeClients) {
                if (client.getLastSeenAt() == null || client.getLastSeenAt().isBefore(staleCutoff)) {
                    client.setActive(false);
                    clientNodeRepository.save(client);
                    log.info("Marked stale client inactive: {} ({})", client.getClientName(), client.getVirtualIp());
                }
            }
            // Delete group only if abandoned for 24 hours (no clients at all)
            List<ClientNode> allClients = clientNodeRepository.findByNetworkGroup(group);
            boolean allOld = allClients.stream().allMatch(c ->
                c.getLastSeenAt() == null || c.getLastSeenAt().isBefore(deleteCutoff));
            if (!allClients.isEmpty() && allOld) {
                log.info("Auto-deleting abandoned group: {}", group.getGroupName());
                clientNodeRepository.deleteAll(allClients);
                networkGroupRepository.delete(group);
            }
        }
    }

    /**
     * Client heartbeat — updates lastSeenAt to keep the client active.
     */
    @Transactional
    public void heartbeat(String groupName, String clientName) {
        networkGroupRepository.findByGroupNameAndActiveTrue(groupName).ifPresent(group -> {
            // Search ALL clients (not just active) so we can re-activate after timeout
            clientNodeRepository.findByNetworkGroup(group).stream()
                    .filter(c -> c.getClientName().equals(clientName))
                    .findFirst()
                    .ifPresent(client -> {
                        client.setLastSeenAt(LocalDateTime.now());
                        client.setActive(true);
                        clientNodeRepository.save(client);
                    });
        });
    }

    /**
     * Client disconnect — marks client as inactive immediately.
     */
    @Transactional
    public void leave(String groupName, String clientName) {
        networkGroupRepository.findByGroupNameAndActiveTrue(groupName).ifPresent(group -> {
            clientNodeRepository.findByNetworkGroupAndActiveTrue(group).stream()
                    .filter(c -> c.getClientName().equals(clientName))
                    .findFirst()
                    .ifPresent(client -> {
                        client.setActive(false);
                        clientNodeRepository.save(client);
                        log.info("Client {} left group {}", clientName, groupName);
                    });
        });
    }

    /**
     * Get active clients in a group (for peer discovery)
     */
    @Transactional(readOnly = true)
    public List<ClientInfo> getActiveClients(String groupName) {
        return networkGroupRepository.findByGroupNameAndActiveTrue(groupName)
                .map(group -> clientNodeRepository.findByNetworkGroupAndActiveTrue(group).stream()
                        .map(node -> ClientInfo.builder()
                                .clientName(node.getClientName())
                                .virtualIp(node.getVirtualIp())
                                .lastSeenAt(node.getLastSeenAt() != null ? node.getLastSeenAt().toString() : null)
                                .build())
                        .collect(Collectors.toList()))
                .orElse(Collections.emptyList());
    }

    /**
     * Constant-time string comparison to prevent timing attacks.
     */
    private boolean constantTimeEquals(String a, String b) {
        if (a == null || b == null) return false;
        byte[] ab = a.getBytes();
        byte[] bb = b.getBytes();
        if (ab.length != bb.length) return false;
        int result = 0;
        for (int i = 0; i < ab.length; i++) result |= ab[i] ^ bb[i];
        return result == 0;
    }
}
