package com.eznebula.controller;

import com.eznebula.domain.entity.NetworkGroup;
import com.eznebula.domain.repository.NetworkGroupRepository;
import com.eznebula.dto.response.ApiResponse;
import com.eznebula.service.IpAllocationService;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.util.List;
import java.util.UUID;

/**
 * Admin controller for managing network groups
 * In production, this should be protected with proper authentication
 */
@Slf4j
@RestController
@RequestMapping("/api/v1/admin/groups")
@RequiredArgsConstructor
public class AdminController {

    private final NetworkGroupRepository networkGroupRepository;
    private final IpAllocationService ipAllocationService;

    /**
     * Create a new network group
     */
    @PostMapping
    public ResponseEntity<ApiResponse<NetworkGroup>> createGroup(
            @RequestParam String groupName,
            @RequestParam String cidrBlock,
            @RequestParam(required = false) String description,
            @RequestParam(required = false) String joinToken) {

        log.info("Creating network group: {} with CIDR: {}", groupName, cidrBlock);

        // Check if group already exists
        if (networkGroupRepository.existsByGroupName(groupName)) {
            return ResponseEntity.badRequest()
                    .body(ApiResponse.error("Group name already exists"));
        }

        // Use provided token or leave null for an open group
        String token = (joinToken != null && !joinToken.isBlank())
                ? joinToken
                : null;

        // Calculate starting IP for the CIDR block
        long startingIp = ipAllocationService.calculateStartingIp(cidrBlock);

        // Create network group
        NetworkGroup group = NetworkGroup.builder()
                .groupName(groupName)
                .joinToken(token)
                .cidrBlock(cidrBlock)
                .nextIpAddress(startingIp)
                .description(description)
                .active(true)
                .build();

        NetworkGroup savedGroup = networkGroupRepository.save(group);

        log.info("Network group created: {} with join token: {}", groupName, joinToken);

        return ResponseEntity.ok(ApiResponse.success(
                "Network group created successfully",
                savedGroup
        ));
    }

    /**
     * List all network groups
     */
    @GetMapping
    public ResponseEntity<ApiResponse<List<NetworkGroup>>> listGroups() {
        List<NetworkGroup> groups = networkGroupRepository.findAll();
        return ResponseEntity.ok(ApiResponse.success(groups));
    }

    /**
     * Get a specific network group
     */
    @GetMapping("/{groupName}")
    public ResponseEntity<ApiResponse<NetworkGroup>> getGroup(@PathVariable String groupName) {
        return networkGroupRepository.findByGroupName(groupName)
                .map(group -> ResponseEntity.ok(ApiResponse.success(group)))
                .orElse(ResponseEntity.notFound().build());
    }

    /**
     * Delete a network group
     */
    @DeleteMapping("/{groupName}")
    public ResponseEntity<ApiResponse<Void>> deleteGroup(@PathVariable String groupName) {
        return networkGroupRepository.findByGroupName(groupName)
                .map(group -> {
                    networkGroupRepository.delete(group);
                    log.info("Network group deleted: {}", groupName);
                    ApiResponse<Void> response = ApiResponse.<Void>builder()
                            .success(true)
                            .message("Network group deleted")
                            .build();
                    return ResponseEntity.ok(response);
                })
                .orElse(ResponseEntity.notFound().build());
    }
}
