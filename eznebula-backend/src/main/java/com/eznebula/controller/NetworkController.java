package com.eznebula.controller;

import com.eznebula.dto.request.JoinNetworkRequest;
import com.eznebula.dto.response.ApiResponse;
import com.eznebula.dto.response.ClientInfo;
import com.eznebula.dto.response.JoinNetworkResponse;
import com.eznebula.service.NetworkService;
import jakarta.validation.Valid;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.util.List;
import java.util.Map;

/**
 * REST controller for network operations
 */
@Slf4j
@RestController
@RequestMapping("/api/v1")
@RequiredArgsConstructor
public class NetworkController {

    private final NetworkService networkService;

    /**
     * Join a network group
     * This is the main API endpoint that clients call to join a network
     *
     * POST /api/v1/join
     * Request body: JoinNetworkRequest (group_name, join_token, client_pub_key, client_name)
     * Response: JoinNetworkResponse (virtual_ip, certificates, lighthouse info)
     */
    @PostMapping("/join")
    public ResponseEntity<ApiResponse<JoinNetworkResponse>> joinNetwork(
            @Valid @RequestBody JoinNetworkRequest request) {

        log.info("Received join request for group: {} from client: {}",
                request.getGroupName(), request.getClientName());

        JoinNetworkResponse response = networkService.joinNetwork(request);

        return ResponseEntity.ok(ApiResponse.success(
                "Successfully joined network",
                response
        ));
    }

    /**
     * Get active clients in a group (for peer discovery)
     */
    @GetMapping("/groups/{groupName}/clients")
    public ResponseEntity<ApiResponse<List<ClientInfo>>> getGroupClients(
            @PathVariable String groupName) {
        List<ClientInfo> clients = networkService.getActiveClients(groupName);
        return ResponseEntity.ok(ApiResponse.success(clients.size() + " clients online", clients));
    }

    /**
     * Client heartbeat — keeps the client marked as active.
     * POST /api/v1/heartbeat
     */
    @PostMapping("/heartbeat")
    public ResponseEntity<ApiResponse<String>> heartbeat(@RequestBody java.util.Map<String, String> body) {
        String groupName = body.get("groupName");
        String clientName = body.get("clientName");
        networkService.heartbeat(groupName, clientName);
        return ResponseEntity.ok(ApiResponse.success("ok"));
    }

    /**
     * Client disconnect — marks client as inactive immediately.
     * POST /api/v1/leave
     */
    @PostMapping("/leave")
    public ResponseEntity<ApiResponse<String>> leave(@RequestBody java.util.Map<String, String> body) {
        String groupName = body.get("groupName");
        String clientName = body.get("clientName");
        networkService.leave(groupName, clientName);
        return ResponseEntity.ok(ApiResponse.success("ok"));
    }

    /**
     * Health check endpoint
     */
    @GetMapping("/health")
    public ResponseEntity<ApiResponse<String>> health() {
        return ResponseEntity.ok(ApiResponse.success("EZNebula server is running"));
    }
}
