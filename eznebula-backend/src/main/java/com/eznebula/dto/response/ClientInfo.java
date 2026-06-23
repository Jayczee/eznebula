package com.eznebula.dto.response;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class ClientInfo {
    private String clientName;
    private String virtualIp;
    private String lastSeenAt;
}
