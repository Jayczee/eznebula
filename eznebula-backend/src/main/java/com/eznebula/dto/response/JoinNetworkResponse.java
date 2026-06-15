package com.eznebula.dto.response;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * Client join response DTO
 * Contains everything the client needs to connect to the network
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class JoinNetworkResponse {

    /**
     * Allocated virtual IP address with CIDR (e.g., "10.168.1.5/24")
     */
    private String virtualIpWithCidr;

    /**
     * Signed client certificate (PEM format)
     */
    private String clientCertificate;

    /**
     * CA certificate (PEM format)
     */
    private String caCertificate;

    /**
     * Lighthouse public IP address
     */
    private String lighthouseIp;

    /**
     * Lighthouse port
     */
    private Integer lighthousePort;

    /**
     * Network group CIDR block (for routing configuration)
     */
    private String networkCidr;

    /**
     * Success message
     */
    private String message;
}
