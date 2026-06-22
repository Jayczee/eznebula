package com.eznebula.service;

import com.eznebula.config.EzNebulaProperties;
import jakarta.annotation.PostConstruct;
import jakarta.annotation.PreDestroy;
import lombok.extern.slf4j.Slf4j;
import org.bouncycastle.crypto.AsymmetricCipherKeyPair;
import org.bouncycastle.crypto.generators.X25519KeyPairGenerator;
import org.bouncycastle.crypto.params.X25519KeyGenerationParameters;
import org.springframework.stereotype.Service;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.security.SecureRandom;
import java.util.Base64;
import java.util.concurrent.TimeUnit;

/**
 * Manages a Nebula lighthouse process that starts alongside the backend.
 * Generates its own X25519 keypair and node certificate signed by the CA.
 */
@Slf4j
@Service
public class LighthouseService {

    private static final String LIGHTHOUSE_NAME = "eznebula-lighthouse";

    private static final String X25519_PRIV_HEADER = "-----BEGIN NEBULA X25519 PRIVATE KEY-----";
    private static final String X25519_PRIV_FOOTER = "-----END NEBULA X25519 PRIVATE KEY-----";
    private static final String X25519_PUB_HEADER  = "-----BEGIN NEBULA X25519 PUBLIC KEY-----";
    private static final String X25519_PUB_FOOTER  = "-----END NEBULA X25519 PUBLIC KEY-----";

    private final EzNebulaProperties properties;
    private final NebulaCertService certService;

    private Process lighthouseProcess;
    private Path lhCertPath;
    private Path lhKeyPath;
    private Path lhConfigPath;

    public LighthouseService(EzNebulaProperties properties, NebulaCertService certService) {
        this.properties = properties;
        this.certService = certService;
    }

    @PostConstruct
    public void start() {
        try {
            log.info("=== Lighthouse initializing ===");

            certService.ensureCA();
            log.info("CA ready: {}", certService.getCaCertPath());

            Path configDir = Paths.get(System.getProperty("user.home"), ".eznebula");
            Files.createDirectories(configDir);
            this.lhCertPath = configDir.resolve("lighthouse.crt");
            this.lhKeyPath = configDir.resolve("lighthouse.key");
            this.lhConfigPath = configDir.resolve("lighthouse-config.yml");

            // Generate X25519 keypair + sign node cert if needed
            if (!Files.exists(lhCertPath) || !Files.exists(lhKeyPath)) {
                log.info("Generating lighthouse X25519 keypair + node certificate...");
                generateLighthouseKeyAndCert();
            }

            writeConfig();
            startProcess();

            Thread.sleep(1000);
            if (lighthouseProcess != null && !lighthouseProcess.isAlive()) {
                log.error("Lighthouse exited immediately, code: {}", lighthouseProcess.exitValue());
            } else {
                log.info("=== Lighthouse running on port {} ===", properties.getLighthouse().getPort());
            }
        } catch (Exception e) {
            log.error("Lighthouse startup FAILED: {}", e.getMessage(), e);
        }
    }

    @PreDestroy
    public void stop() {
        if (lighthouseProcess != null && lighthouseProcess.isAlive()) {
            lighthouseProcess.destroy();
            try { lighthouseProcess.waitFor(5, TimeUnit.SECONDS); }
            catch (InterruptedException e) { Thread.currentThread().interrupt(); }
            if (lighthouseProcess.isAlive()) lighthouseProcess.destroyForcibly();
        }
    }

    /** Generate X25519 keypair using BouncyCastle, write PEM, sign with CA */
    private void generateLighthouseKeyAndCert() throws Exception {
        // 1. Generate X25519 key
        X25519KeyPairGenerator gen = new X25519KeyPairGenerator();
        gen.init(new X25519KeyGenerationParameters(new SecureRandom()));
        AsymmetricCipherKeyPair pair = gen.generateKeyPair();

        byte[] privBytes = ((org.bouncycastle.crypto.params.X25519PrivateKeyParameters) pair.getPrivate()).getEncoded();
        byte[] pubBytes  = ((org.bouncycastle.crypto.params.X25519PublicKeyParameters) pair.getPublic()).getEncoded();
        Base64.Encoder b64 = Base64.getEncoder();

        String privPem = X25519_PRIV_HEADER + "\n" + b64.encodeToString(privBytes) + "\n" + X25519_PRIV_FOOTER + "\n";
        String pubPem  = X25519_PUB_HEADER  + "\n" + b64.encodeToString(pubBytes)  + "\n" + X25519_PUB_FOOTER  + "\n";

        Files.writeString(lhKeyPath, privPem, StandardCharsets.UTF_8);
        log.info("  Lighthouse X25519 key written: {}", lhKeyPath);

        // 2. Sign node certificate with CA
        Path tempPub = Files.createTempFile("lh-pub-", ".pem");
        Files.writeString(tempPub, pubPem, StandardCharsets.UTF_8);
        try {
            String lhIp = properties.getLighthouse().getNebulaIp();
            String lhCidr = lhIp + "/24";
            certService.ensureCA();
            runNebulaCert("sign",
                    "-ca-crt", certService.getCaCertPath().toString(),
                    "-ca-key", certService.getCaKeyPath().toString(),
                    "-name", LIGHTHOUSE_NAME,
                    "-ip", lhCidr,
                    "-in-pub", tempPub.toString(),
                    "-out-crt", lhCertPath.toString());
            log.info("  Lighthouse node cert signed: {}", lhCertPath);
        } finally {
            deleteQuietly(tempPub);
        }
    }

    private void writeConfig() throws IOException {
        int port = properties.getLighthouse().getPort();
        String yml = String.format("""
                pki:
                  ca: "%s"
                  cert: "%s"
                  key: "%s"
                lighthouse:
                  am_lighthouse: true
                relay:
                  am_relay: true
                  use_relays: true
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
                certService.getCaCertPath().toString().replace('\\', '/'),
                lhCertPath.toString().replace('\\', '/'),
                lhKeyPath.toString().replace('\\', '/'),
                port);
        Files.writeString(lhConfigPath, yml, StandardCharsets.UTF_8);
        log.info("Lighthouse config: {}", lhConfigPath);
    }

    private void startProcess() throws IOException {
        String nebulaBin = findBinary();
        ProcessBuilder pb = new ProcessBuilder(nebulaBin, "-config", lhConfigPath.toString());
        pb.redirectErrorStream(true);
        this.lighthouseProcess = pb.start();

        Thread reader = new Thread(() -> {
            try (BufferedReader br = new BufferedReader(
                    new InputStreamReader(lighthouseProcess.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = br.readLine()) != null) {
                    if      (line.contains("level=error"))   log.warn("[lh] {}", line);
                    else if (line.contains("level=warning")) log.warn("[lh] {}", line);
                    else                                     log.info("[lh] {}", line);
                }
            } catch (IOException ignored) {}
            try {
                int exit = lighthouseProcess.waitFor();
                if (exit != 0) log.error("[lh] exited: {}", exit);
                else log.info("[lh] stopped normally");
            } catch (InterruptedException e) { Thread.currentThread().interrupt(); }
        }, "lh-reader");
        reader.setDaemon(true);
        reader.start();
        log.info("Lighthouse PID: {}", lighthouseProcess.pid());
    }

    private void runNebulaCert(String... args) throws Exception {
        String certBin = findCertBinary();
        String[] cmd = new String[args.length + 1];
        cmd[0] = certBin;
        System.arraycopy(args, 0, cmd, 1, args.length);
        Process p = new ProcessBuilder(cmd).redirectErrorStream(true).start();
        StringBuilder out = new StringBuilder();
        try (BufferedReader r = new BufferedReader(new InputStreamReader(p.getInputStream()))) {
            String l; while ((l = r.readLine()) != null) out.append(l).append("\n");
        }
        int exit = p.waitFor();
        if (exit != 0) throw new RuntimeException("nebula-cert failed: " + out.toString().trim());
    }

    private String findBinary() {
        return findExe("nebula");
    }
    private String findCertBinary() {
        return findExe("nebula-cert");
    }
    private String findExe(String name) {
        boolean isWin = System.getProperty("os.name","").toLowerCase().contains("win");
        String exeName = isWin ? name + ".exe" : name;
        Path bundled = Paths.get(exeName);
        if (Files.isRegularFile(bundled) && !isWin || Files.isExecutable(bundled)) return bundled.toAbsolutePath().toString();
        return exeName;
    }

    private void deleteQuietly(Path p) {
        try { Files.deleteIfExists(p); } catch (IOException ignored) {}
    }
}
