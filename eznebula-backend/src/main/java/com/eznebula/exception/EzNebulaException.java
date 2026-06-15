package com.eznebula.exception;

/**
 * Base exception for EZNebula business logic errors
 */
public class EzNebulaException extends RuntimeException {

    private final String errorCode;

    public EzNebulaException(String message) {
        super(message);
        this.errorCode = null;
    }

    public EzNebulaException(String message, String errorCode) {
        super(message);
        this.errorCode = errorCode;
    }

    public EzNebulaException(String message, Throwable cause) {
        super(message, cause);
        this.errorCode = null;
    }

    public EzNebulaException(String message, String errorCode, Throwable cause) {
        super(message, cause);
        this.errorCode = errorCode;
    }

    public String getErrorCode() {
        return errorCode;
    }
}
