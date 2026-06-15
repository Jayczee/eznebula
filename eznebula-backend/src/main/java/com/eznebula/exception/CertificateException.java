package com.eznebula.exception;

/**
 * Exception thrown when certificate operations fail
 */
public class CertificateException extends EzNebulaException {

    public CertificateException(String message) {
        super(message, "CERT_ERROR");
    }

    public CertificateException(String message, Throwable cause) {
        super(message, "CERT_ERROR", cause);
    }
}
