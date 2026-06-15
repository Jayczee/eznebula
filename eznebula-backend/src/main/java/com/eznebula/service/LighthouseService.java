package com.eznebula.service;

import com.eznebula.config.EzNebulaProperties;
import jakarta.annotation.PostConstruct;
import jakarta.annotation.PreDestroy;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.concurrent.TimeUnit;

/**
 * Manages a Nebula lighthouse process that starts alongside the backend.
 * The lighthouse is the central rendezvous point for all Nebula nodes.
 */
@Slf4j
@Service
public class LighthouseService {

    private final EzNebulaProperties properties;
    private final NebulaCertService certService;

    private Process lighthouseProcess;
    private Path configPath;

    public LighthouseService(EzNebulaProperties properties, NebulaCertService certService) {
        this.properties = properties;
        this.certService = certService;
    }

    @PostConstruct
    public void start() {
        try {
            log.info("=== Lighthouse initializing ===");

            // Ensure CA exists
            log.info("Step 1/3: Ensuring CA cert/key exist...");
            certService.ensureCA();
            log.info("  CA cert: {}", certService.getCaCertPath().toAbsolutePath());
            log.info("  CA key:  {}", certService.getCaKeyPath().toAbsolutePath());
            log.info("  CA exists: {} / {}",
                    Files.exists(certService.getCaCertPath()),
                    Files.exists(certService.getCaKeyPath()));

            // Generate lighthouse config
            log.info("Step 2/3: Writing lighthouse config...");
            this.configPath = writeLighthouseConfig();

            // Print config content
            String configContent = Files.readString(configPath, StandardCharsets.UTF_8);
            log.info("  Config path: {}", configPath.toAbsolutePath());
            log.info("  Config content:\n{}", configContent);

            // Start the lighthouse process
            log.info("Step 3/3: Starting nebula lighthouse process...");
            startLighthouseProcess(configPath);

            // Short wait to catch immediate failures
            Thread.sleep(1000);
            if (lighthouseProcess != null && !lighthouseProcess.isAlive()) {
                int exitCode = lighthouseProcess.exitValue();
                log.error("Lighthouse exited immediately with code: {}", exitCode);
            } else if (lighthouseProcess != null) {
                log.info("Lighthouse is alive, PID: {}", lighthouseProcess.pid());
            }

            log.info("=== Lighthouse started on port {} ===", properties.getLighthouse().getPort());

        } catch (Exception e) {
            log.error("=== Lighthouse startup FAILED: {} ===", e.getMessage(), e);
        }
    }

    @PreDestroy
    public void stop() {
        if (lighthouseProcess != null && lighthouseProcess.isAlive()) {
            log.info("Stopping lighthouse (PID: {})...", lighthouseProcess.pid());
            lighthouseProcess.destroy();
            try {
                boolean terminated = lighthouseProcess.waitFor(5, TimeUnit.SECONDS);
                log.info("Lighthouse terminated gracefully: {}", terminated);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            }
            if (lighthouseProcess.isAlive()) {
                lighthouseProcess.destroyForcibly();
                log.info("Lighthouse forcefully stopped");
            }
            log.info("Lighthouse stopped, exit code: {}", lighthouseProcess.exitValue());
        }
    }

    private Path writeLighthouseConfig() throws IOException {
        Path configDir = Paths.get(System.getProperty("user.home"), ".eznebula");
        Files.createDirectories(configDir);

        Path path = configDir.resolve("lighthouse-config.yml");

        String caPath = certService.getCaCertPath().toString().replace('\\', '/');
        String caKeyPath = certService.getCaKeyPath().toString().replace('\\', '/');
        int port = properties.getLighthouse().getPort();

        String yml = String.format("""
                pki:
                  ca: "%s"
                  cert: "%s"
                  key: "%s"
                lighthouse:
                  am_lighthouse: true
                listen:
                  host: 0.0.0.0
                  port: %d
                tun:
                  disabled: true
                logging:
                  level: info
                  format: text
                firewall:
                  outbound:
                    - port: any
                      proto: any
                      host: any
                  inbound:
                    - port: any
                      proto: any
                      host: any
                """,
                caPath, caPath, caKeyPath, port);

        Files.writeString(path, yml, StandardCharsets.UTF_8);
        return path;
    }

    private void startLighthouseProcess(Path configPath) throws IOException {
        String nebulaBin = resolveBinary();
        log.info("  Binary: {}", nebulaBin);

        Path nebulaExe = Paths.get(nebulaBin);
        Path workDir = nebulaExe.getParent();
        log.info("  Work dir: {}", workDir != null ? workDir.toAbsolutePath() : "N/A");

        ProcessBuilder pb = new ProcessBuilder(nebulaBin, "-config", configPath.toString());
        if (workDir != null) {
            pb.directory(workDir.toFile());
        }
        pb.redirectErrorStream(true);

        this.lighthouseProcess = pb.start();
        log.info("  PID: {}", lighthouseProcess.pid());

        // Background thread to read lighthouse stdout/stderr
        Thread reader = new Thread(() -> {
            try (BufferedReader br = new BufferedReader(
                    new InputStreamReader(lighthouseProcess.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = br.readLine()) != null) {
                    // Route nebula output at INFO level for visibility
                    if (line.contains("level=error") || line.contains("level=warning")) {
                        log.warn("[lighthouse] {}", line);
                    } else if (line.contains("level=info")) {
                        log.info("[lighthouse] {}", line);
                    } else {
                        log.info("[lighthouse] {}", line);
                    }
                }
            } catch (IOException e) {
                log.debug("lighthouse reader stream closed: {}", e.getMessage());
            }

            // Process exited — log the exit code
            try {
                int exit = lighthouseProcess.waitFor();
                if (exit == 0) {
                    log.info("[lighthouse] Process exited normally (code 0)");
                } else {
                    log.error("[lighthouse] Process exited with code: {}", exit);
                }
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            }
        }, "lh-reader");
        reader.setDaemon(true);
        reader.start();
    }

    private String resolveBinary() {
        String bin = System.getProperty("os.name", "").toLowerCase().contains("win")
                ? "nebula.exe" : "nebula";

        // 1) Next to JAR
        Path bundled = Paths.get(bin);
        if (Files.isRegularFile(bundled)) {
            log.info("  Found binary next to JAR: {}", bundled.toAbsolutePath());
            return bundled.toAbsolutePath().toString();
        }

        // 2) PATH
        log.info("  Binary not found next to JAR, using PATH: {}", bin);
        return bin;
    }
}
