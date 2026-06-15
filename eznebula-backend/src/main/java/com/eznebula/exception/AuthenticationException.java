package com.eznebula.exception;

/**
 * Exception thrown when authentication fails
 */
public class AuthenticationException extends EzNebulaException {

    public AuthenticationException(String message) {
        super(message, "AUTH_FAILED");
    }

    public AuthenticationException(String message, Throwable cause) {
        super(message, "AUTH_FAILED", cause);
    }
}
