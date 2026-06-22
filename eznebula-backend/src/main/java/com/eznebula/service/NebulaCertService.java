package com.eznebula.service;

import com.eznebula.config.EzNebulaProperties;
import com.eznebula.exception.CertificateException;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import jakarta.annotation.PostConstruct;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;

/**
 * Service for managing Nebula certificates using nebula-cert binary.
 * CA generation is lazy — it only runs when the first certificate operation
 * is requested, not at application startup.
 */
@Slf4j
@Service
public class NebulaCertService {

    private final EzNebulaProperties properties;

    private Path caStoragePath;
    private Path caCertPath;
    private Path caKeyPath;

    // Resolved binary path (may be an absolute path to the bundled binary)
    private Path certBinaryPath;

    public NebulaCertService(EzNebulaProperties properties) {
        this.properties = properties;
    }

    @PostConstruct
    public void init() {
        // Initialize CA storage paths
        this.caStoragePath = Paths.get(properties.getCa().getStoragePath());
        this.caCertPath = caStoragePath.resolve(properties.getCa().getCertFile());
        this.caKeyPath = caStoragePath.resolve(properties.getCa().getKeyFile());

        // Resolve nebula-cert binary. Look next to the JAR first (bundled),
        // then fall back to PATH.
        this.certBinaryPath = resolveCertBinary();

        try {
            // Create CA storage directory
            Files.createDirectories(caStoragePath);
            log.info("CA storage path: {}", caStoragePath.toAbsolutePath());
        } catch (IOException e) {
            throw new CertificateException("Failed to create CA storage directory", e);
        }
    }

    /**
     * Resolve the nebula-cert binary location.
     * Tries: 1) ./nebula-cert(.exe) next to the JAR, 2) system PATH.
     */
    private Path resolveCertBinary() {
        String binaryName = properties.getNebula().getCertBinary();
        if (binaryName == null) {
            binaryName = "nebula-cert";
        }

        boolean isWindows = System.getProperty("os.name", "").toLowerCase().contains("win");
        String exeName = isWindows ? binaryName + ".exe" : binaryName;

        // 1) Bundled: look next to the running JAR
        try {
            Path jarDir = Paths.get(".").toAbsolutePath();
            Path bundled = jarDir.resolve(exeName);
            if (Files.isRegularFile(bundled) && Files.isExecutable(bundled)) {
                log.info("Using bundled nebula-cert: {}", bundled.toAbsolutePath());
                return bundled;
            }
        } catch (Exception ignored) {}

        // 2) System PATH
        log.info("nebula-cert not found bundled, using PATH: {}", exeName);
        return Paths.get(exeName);
    }

    /**
     * Ensure CA certificate and key exist, generating them if needed.
     * Called lazily — not during startup.
     */
    public synchronized void ensureCA() {
        if (Files.exists(caCertPath) && Files.exists(caKeyPath)) {
            return;
        }
        log.info("CA certificates not found, generating new CA");
        generateCACertificate();
    }

    /**
     * Generate CA certificate and key
     */
    private void generateCACertificate() {
        log.info("Generating CA certificate and key");

        List<String> command = new ArrayList<>();
        command.add(certBinaryPath.toString());
        command.add("ca");
        command.add("-name");
        command.add(properties.getCa().getOrganization());
        command.add("-out-crt");
        command.add(caCertPath.toString());
        command.add("-out-key");
        command.add(caKeyPath.toString());
        command.add("-duration");
        command.add(properties.getCa().getDuration());

        executeCommand(command, "Failed to generate CA certificate");
        log.info("CA certificate generated successfully");
    }

    /**
     * Sign a client certificate using the CA
     */
    public String signCertificate(String clientName, String clientPublicKey,
                                   String virtualIp, int cidrSuffix, List<String> groups) {
        ensureCA();

        log.debug("Signing certificate for client: {}, IP: {}/{}", clientName, virtualIp, cidrSuffix);

        Path tempPubKeyPath = null;
        Path tempCertPath = null;

        try {
            tempPubKeyPath = Files.createTempFile("client-pub-", ".key");
            Files.writeString(tempPubKeyPath, clientPublicKey, StandardCharsets.UTF_8);

            // Get a unique path but don't pre-create the file —
            // nebula-cert refuses to overwrite an existing file.
            tempCertPath = Files.createTempFile("client-cert-", ".crt");
            Files.deleteIfExists(tempCertPath);

            List<String> command = new ArrayList<>();
            command.add(certBinaryPath.toString());
            command.add("sign");
            command.add("-ca-crt");
            command.add(caCertPath.toString());
            command.add("-ca-key");
            command.add(caKeyPath.toString());
            command.add("-name");
            command.add(clientName);
            command.add("-ip");
            command.add(virtualIp + "/" + cidrSuffix);
            // 同时签发灯塔所在网络，使客户端能与 relay 建立 layer 3 数据隧道
            command.add("-ip");
            command.add(properties.getLighthouse().getNebulaIp() + "/24");
            command.add("-in-pub");
            command.add(tempPubKeyPath.toString());
            command.add("-out-crt");
            command.add(tempCertPath.toString());
            command.add("-duration");
            command.add("8760h"); // 1 year, re-signed on every reconnect anyway

            if (groups != null && !groups.isEmpty()) {
                command.add("-groups");
                command.add(String.join(",", groups));
            }

            executeCommand(command, "Failed to sign certificate");

            String certificate = Files.readString(tempCertPath, StandardCharsets.UTF_8);
            log.debug("Certificate signed successfully for {}", clientName);
            return certificate;

        } catch (IOException e) {
            throw new CertificateException("Failed to sign certificate for " + clientName, e);
        } finally {
            deleteQuietly(tempPubKeyPath);
            deleteQuietly(tempCertPath);
        }
    }

    /** Get CA certificate content */
    public String getCACertificate() {
        ensureCA();
        try {
            return Files.readString(caCertPath, StandardCharsets.UTF_8);
        } catch (IOException e) {
            throw new CertificateException("Failed to read CA certificate", e);
        }
    }

    /** Get CA certificate file path */
    public Path getCaCertPath() { return caCertPath; }

    /** Get CA private key file path */
    public Path getCaKeyPath() { return caKeyPath; }

    /**
     * Execute a command and handle the output
     */
    private void executeCommand(List<String> command, String errorMessage) {
        try {
            log.debug("Executing command: {}", String.join(" ", command));

            ProcessBuilder pb = new ProcessBuilder(command);
            pb.redirectErrorStream(true);
            Process process = pb.start();

            StringBuilder output = new StringBuilder();
            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    output.append(line).append("\n");
                    log.debug("nebula-cert: {}", line);
                }
            }

            int exitCode = process.waitFor();

            if (exitCode != 0) {
                throw new CertificateException(
                    String.format("%s (exit: %d) %s",
                                  errorMessage, exitCode, output.toString().trim()));
            }

        } catch (IOException e) {
            throw new CertificateException(
                errorMessage + " — is nebula-cert installed? " + e.getMessage(), e);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new CertificateException(errorMessage, e);
        }
    }

    private void deleteQuietly(Path path) {
        if (path != null) {
            try { Files.deleteIfExists(path); }
            catch (IOException e) { log.warn("Failed to delete temp file: {}", path, e); }
        }
    }
}
