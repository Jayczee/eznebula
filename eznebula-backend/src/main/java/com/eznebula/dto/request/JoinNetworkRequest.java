package com.eznebula.dto.request;

import jakarta.validation.constraints.NotBlank;
import jakarta.validation.constraints.Pattern;
import jakarta.validation.constraints.Size;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

/**
 * Client join request DTO
 * Used when a client wants to join a network group
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class JoinNetworkRequest {

    /**
     * Network group name to join
     */
    @NotBlank(message = "Group name is required")
    @Size(min = 3, max = 64, message = "Group name must be between 3 and 64 characters")
    @Pattern(regexp = "^[a-zA-Z0-9_-]+$", message = "Group name can only contain letters, numbers, hyphens, and underscores")
    private String groupName;

    /**
     * Join token for authentication (optional for open groups)
     */
    @Size(max = 128, message = "Join token is too long")
    private String joinToken;

    /**
     * Client's public key (Ed25519)
     */
    @NotBlank(message = "Client public key is required")
    @Size(max = 512, message = "Public key is too long")
    private String clientPublicKey;

    /**
     * Client device name
     */
    @NotBlank(message = "Client name is required")
    @Size(min = 1, max = 128, message = "Client name must be between 1 and 128 characters")
    @Pattern(regexp = "^[a-zA-Z0-9_.-]+$", message = "Client name can only contain letters, numbers, dots, hyphens, and underscores")
    private String clientName;
}
