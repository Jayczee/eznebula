package com.eznebula.config;

import lombok.Data;
import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.context.annotation.Configuration;

/**
 * EZNebula configuration properties
 */
@Data
@Configuration
@ConfigurationProperties(prefix = "eznebula")
public class EzNebulaProperties {

    private CaProperties ca = new CaProperties();
    private NebulaProperties nebula = new NebulaProperties();
    private LighthouseProperties lighthouse = new LighthouseProperties();
    private SecurityProperties security = new SecurityProperties();

    @Data
    public static class CaProperties {
        /**
         * CA certificate and key storage path
         */
        private String storagePath;

        /**
         * CA certificate filename
         */
        private String certFile = "ca.crt";

        /**
         * CA private key filename
         */
        private String keyFile = "ca.key";

        /**
         * Organization name in certificate
         */
        private String organization = "eznebula";

        /**
         * Certificate validity duration (e.g., "87600h" for 10 years)
         */
        private String duration = "87600h";
    }

    @Data
    public static class NebulaProperties {
        /**
         * Path to nebula-cert binary
         */
        private String certBinary = "nebula-cert";
    }

    @Data
    public static class LighthouseProperties {
        /**
         * Lighthouse public IP address (or hostname) that clients use to reach the lighthouse.
         * Must be reachable from clients. Example: "www.jayczee.cn" or "1.2.3.4"
         */
        private String publicIp;

        /**
         * Lighthouse Nebula virtual IP (within the VPN overlay network).
         * This is the IP clients use in static_host_map key and lighthouse.hosts.
         */
        private String nebulaIp = "10.168.255.1";

        /**
         * Lighthouse port (UDP)
         */
        private Integer port = 4242;
    }

    @Data
    public static class SecurityProperties {
        /**
         * Minimum join token length
         */
        private Integer tokenMinLength = 8;
    }
}
