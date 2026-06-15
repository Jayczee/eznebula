package com.eznebula;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.core.env.StandardEnvironment;
import org.springframework.scheduling.annotation.EnableScheduling;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;

@SpringBootApplication
@EnableScheduling
public class EzNebulaApplication {

    public static void main(String[] args) {
        // Ensure the SQLite database directory exists BEFORE Spring/Hibernate
        // tries to connect — the sqlite JDBC driver does NOT auto-create parent
        // directories and HikariCP pools are created eagerly during context init.
        ensureDatabaseDirectoryExists();

        SpringApplication.run(EzNebulaApplication.class, args);
    }

    private static void ensureDatabaseDirectoryExists() {
        // Build a simple Spring Environment just to resolve property placeholders in
        // the datasource URL (${user.home}). We do this before the full application
        // context starts so the directory is ready before HikariCP connects.
        StandardEnvironment env = new StandardEnvironment();

        String url = env.getProperty("spring.datasource.url");
        if (url == null || !url.startsWith("jdbc:sqlite:")) {
            // Fallback: try boot's resolution via the boot property source
            String userHome = System.getProperty("user.home");
            url = "jdbc:sqlite:" + userHome + "/.eznebula/eznebula.db";
        }

        String filePath = url.substring("jdbc:sqlite:".length());
        Path dbPath = Paths.get(filePath);
        Path parentDir = dbPath.getParent();

        if (parentDir != null && Files.notExists(parentDir)) {
            try {
                Files.createDirectories(parentDir);
                System.out.println("[EZNebula] Created database directory: " + parentDir);
            } catch (Exception e) {
                throw new IllegalStateException(
                        "Failed to create database directory: " + parentDir, e);
            }
        }
    }
}
