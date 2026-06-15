package com.eznebula.domain.repository;

import com.eznebula.domain.entity.NetworkGroup;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.data.jpa.repository.Lock;
import org.springframework.stereotype.Repository;

import jakarta.persistence.LockModeType;
import java.util.Optional;

@Repository
public interface NetworkGroupRepository extends JpaRepository<NetworkGroup, Long> {

    /**
     * Find network group by name
     */
    Optional<NetworkGroup> findByGroupName(String groupName);

    /**
     * Find network group by name with pessimistic write lock
     * Used during IP allocation to prevent race conditions
     */
    @Lock(LockModeType.PESSIMISTIC_WRITE)
    Optional<NetworkGroup> findByGroupNameAndActiveTrue(String groupName);

    /**
     * Check if group name exists
     */
    boolean existsByGroupName(String groupName);
}
