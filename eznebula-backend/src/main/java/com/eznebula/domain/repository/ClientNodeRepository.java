package com.eznebula.domain.repository;

import com.eznebula.domain.entity.ClientNode;
import com.eznebula.domain.entity.NetworkGroup;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.stereotype.Repository;

import java.util.List;
import java.util.Optional;

@Repository
public interface ClientNodeRepository extends JpaRepository<ClientNode, Long> {

    /**
     * Find all clients in a network group
     */
    List<ClientNode> findByNetworkGroup(NetworkGroup networkGroup);

    /**
     * Find all active clients in a network group
     */
    List<ClientNode> findByNetworkGroupAndActiveTrue(NetworkGroup networkGroup);

    /**
     * Find client by virtual IP in a group
     */
    Optional<ClientNode> findByNetworkGroupAndVirtualIp(NetworkGroup networkGroup, String virtualIp);

    /**
     * Check if virtual IP is already allocated in a group
     */
    boolean existsByNetworkGroupAndVirtualIp(NetworkGroup networkGroup, String virtualIp);

    /**
     * Find client by public key (for duplicate detection)
     */
    Optional<ClientNode> findByPublicKey(String publicKey);
}
