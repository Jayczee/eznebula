package com.eznebula.service;

import com.eznebula.config.EzNebulaProperties;
import com.eznebula.domain.entity.ClientNode;
import com.eznebula.domain.entity.NetworkGroup;
import com.eznebula.domain.repository.ClientNodeRepository;
import com.eznebula.domain.repository.NetworkGroupRepository;
import com.eznebula.dto.request.JoinNetworkRequest;
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
     * Periodic cleanup: delete groups that have no active clients.
     * Runs every 60 seconds.
     */
    @Scheduled(fixedRate = 60_000)
    @Transactional
    public void cleanupEmptyGroups() {
        List<NetworkGroup> allGroups = networkGroupRepository.findAll();

        for (NetworkGroup group : allGroups) {
            List<ClientNode> clients = clientNodeRepository.findByNetworkGroupAndActiveTrue(group);
            if (clients.isEmpty()) {
                log.info("Auto-deleting empty group: {}", group.getGroupName());
                // Delete all clients (including inactive) before deleting group
                List<ClientNode> allClients = clientNodeRepository.findByNetworkGroup(group);
                clientNodeRepository.deleteAll(allClients);
                networkGroupRepository.delete(group);
            }
        }
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
