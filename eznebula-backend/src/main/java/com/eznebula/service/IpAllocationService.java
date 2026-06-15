package com.eznebula.service;

import com.eznebula.domain.entity.NetworkGroup;
import com.eznebula.domain.repository.ClientNodeRepository;
import com.eznebula.exception.EzNebulaException;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import java.net.InetAddress;
import java.net.UnknownHostException;
import java.nio.ByteBuffer;

/**
 * Service for IP address allocation within network groups
 */
@Slf4j
@Service
@RequiredArgsConstructor
public class IpAllocationService {

    private final ClientNodeRepository clientNodeRepository;

    /**
     * Allocate the next available IP address for a network group
     * Uses pessimistic locking to prevent race conditions
     *
     * @param group Network group (must be locked)
     * @return Allocated IP address as string (e.g., "10.168.1.5")
     */
    public String allocateIp(NetworkGroup group) {
        // Get next IP and increment
        Long currentIp = group.getNextIpAddress();

        // Convert to IP string
        String ipAddress = longToIp(currentIp);

        // Validate IP is within CIDR block
        if (!isIpInCidr(ipAddress, group.getCidrBlock())) {
            throw new EzNebulaException("IP pool exhausted for group: " + group.getGroupName());
        }

        // Check if IP is already allocated (defensive check)
        if (clientNodeRepository.existsByNetworkGroupAndVirtualIp(group, ipAddress)) {
            // Try to recover by finding the next free IP
            ipAddress = findNextFreeIp(group);
        }

        // Increment for next allocation
        group.setNextIpAddress(currentIp + 1);

        return ipAddress;
    }

    /**
     * Find the next free IP in the group (recovery mechanism)
     */
    private String findNextFreeIp(NetworkGroup group) {
        String cidr = group.getCidrBlock();
        String[] parts = cidr.split("/");
        String baseIp = parts[0];
        int prefixLength = Integer.parseInt(parts[1]);

        long baseIpLong = ipToLong(baseIp);
        long maxHosts = (1L << (32 - prefixLength)) - 2; // Exclude network and broadcast

        for (long offset = 1; offset <= maxHosts; offset++) {
            String candidateIp = longToIp(baseIpLong + offset);
            if (!clientNodeRepository.existsByNetworkGroupAndVirtualIp(group, candidateIp)) {
                // Update the group's next IP pointer
                group.setNextIpAddress(baseIpLong + offset + 1);
                return candidateIp;
            }
        }

        throw new EzNebulaException("No available IPs in group: " + group.getGroupName());
    }

    /**
     * Get CIDR suffix from CIDR block (e.g., "10.168.0.0/16" -> 16)
     */
    public int getCidrSuffix(String cidrBlock) {
        String[] parts = cidrBlock.split("/");
        if (parts.length != 2) {
            throw new EzNebulaException("Invalid CIDR block format: " + cidrBlock);
        }
        return Integer.parseInt(parts[1]);
    }

    /**
     * Convert IP address string to long
     */
    private long ipToLong(String ipAddress) {
        try {
            InetAddress inetAddress = InetAddress.getByName(ipAddress);
            byte[] bytes = inetAddress.getAddress();
            return ByteBuffer.wrap(bytes).getInt() & 0xFFFFFFFFL;
        } catch (UnknownHostException e) {
            throw new EzNebulaException("Invalid IP address: " + ipAddress, e);
        }
    }

    /**
     * Convert long to IP address string
     */
    private String longToIp(long ip) {
        return String.format("%d.%d.%d.%d",
                (ip >> 24) & 0xFF,
                (ip >> 16) & 0xFF,
                (ip >> 8) & 0xFF,
                ip & 0xFF);
    }

    /**
     * Check if an IP address is within a CIDR block
     */
    private boolean isIpInCidr(String ipAddress, String cidr) {
        try {
            String[] cidrParts = cidr.split("/");
            String cidrIp = cidrParts[0];
            int prefixLength = Integer.parseInt(cidrParts[1]);

            long ipLong = ipToLong(ipAddress);
            long cidrIpLong = ipToLong(cidrIp);

            // Calculate network mask
            long mask = (0xFFFFFFFFL << (32 - prefixLength)) & 0xFFFFFFFFL;

            // Check if IP is in the same network
            return (ipLong & mask) == (cidrIpLong & mask);

        } catch (Exception e) {
            throw new EzNebulaException("Failed to validate IP against CIDR", e);
        }
    }

    /**
     * Calculate the starting IP for a CIDR block (network address + 1)
     * Skip the network address itself
     */
    public long calculateStartingIp(String cidrBlock) {
        String[] parts = cidrBlock.split("/");
        String baseIp = parts[0];
        return ipToLong(baseIp) + 1; // Skip network address
    }

    /**
     * Auto-assign a /24 subnet for a new group.
     * Base: 10.168.0.0/16, each group gets 10.168.{N}.0/24 where N starts at 1.
     * Scans existing groups and picks the next unused subnet number.
     */
    public String allocateGroupCidr(java.util.List<NetworkGroup> existingGroups, String baseCidr) {
        // baseCidr = "10.168.0.0/16", extract base IP
        String[] parts = baseCidr.split("/");
        String baseIp = parts[0];  // 10.168.0.0

        // Collect used subnet numbers
        java.util.Set<Integer> used = new java.util.HashSet<>();
        for (NetworkGroup g : existingGroups) {
            String cidr = g.getCidrBlock();
            // e.g. "10.168.5.0/24" -> extract 5
            String[] segs = cidr.split("\\.")[2].split("/");
            if (segs.length > 0) {
                try {
                    used.add(Integer.parseInt(segs[0]));
                } catch (NumberFormatException ignored) {}
            }
        }

        // Find next free /24 subnet (max 255 subnets)
        for (int i = 1; i < 255; i++) {
            if (!used.contains(i)) {
                // Build CIDR like "10.168.1.0/24"
                String[] baseSegs = baseIp.split("\\.");
                return baseSegs[0] + "." + baseSegs[1] + "." + i + ".0/24";
            }
        }

        throw new EzNebulaException("No available /24 subnets left in " + baseCidr);
    }
}
