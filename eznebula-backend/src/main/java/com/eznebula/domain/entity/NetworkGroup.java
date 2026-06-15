package com.eznebula.domain.entity;

import jakarta.persistence.*;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;
import org.hibernate.annotations.CreationTimestamp;
import org.hibernate.annotations.UpdateTimestamp;

import java.time.LocalDateTime;

/**
 * Network Group Entity
 * Represents a virtual network group in the Nebula mesh
 */
@Entity
@Table(name = "network_groups", indexes = {
    @Index(name = "idx_group_name", columnList = "groupName", unique = true)
})
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class NetworkGroup {

    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    private Long id;

    /**
     * Unique group name (e.g., "dev-team", "production")
     */
    @Column(nullable = false, unique = true, length = 64)
    private String groupName;

    /**
     * Join token for authentication (null = open group, no token required)
     */
    @Column(length = 128)
    private String joinToken;

    /**
     * CIDR block allocated to this group (e.g., "10.168.0.0/16")
     */
    @Column(nullable = false, length = 32)
    private String cidrBlock;

    /**
     * Next available IP in the pool (for sequential allocation)
     * Stored as integer for easy increment (e.g., 168034816 = 10.168.0.0)
     */
    @Column(nullable = false)
    private Long nextIpAddress;

    /**
     * Group description
     */
    @Column(length = 512)
    private String description;

    /**
     * Whether this group is active
     */
    @Column(nullable = false)
    @Builder.Default
    private Boolean active = true;

    @CreationTimestamp
    @Column(nullable = false, updatable = false)
    private LocalDateTime createdAt;

    @UpdateTimestamp
    @Column(nullable = false)
    private LocalDateTime updatedAt;
}
