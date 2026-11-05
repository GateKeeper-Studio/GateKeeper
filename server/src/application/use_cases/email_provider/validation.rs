use crate::domain::{entities::email_provider::SmtpEncryption, errors::DomainError};

pub fn parse_encryption(value: &str) -> Result<SmtpEncryption, DomainError> {
    match value {
        "starttls" => Ok(SmtpEncryption::StartTls),
        "tls" => Ok(SmtpEncryption::Tls),
        "none" => Ok(SmtpEncryption::None),
        other => Err(DomainError::Validation(format!(
            "invalid encryption '{other}' (expected starttls | tls | none)"
        ))),
    }
}

// SMTP ports are 16-bit and 0 is never valid.
pub fn validate_port(port: i32) -> Result<(), DomainError> {
    if (1..=65535).contains(&port) {
        Ok(())
    } else {
        Err(DomainError::Validation(format!(
            "invalid SMTP port: {port}"
        )))
    }
}
