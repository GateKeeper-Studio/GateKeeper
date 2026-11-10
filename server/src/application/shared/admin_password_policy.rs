// The rules a new platform-admin password must meet, applied wherever one is set
// (`create_admin`, the account page). Follows NIST SP 800-63B: length is what
// matters, there are no composition rules, and any character is allowed. The
// minimum is above the 8 used for end users because these accounts administer
// every tenant of an organization.
use crate::domain::errors::DomainError;

pub const MIN_LENGTH: usize = 12;
// Long enough for any passphrase or generated password; bounded so a request
// cannot hand Argon2 megabytes of input.
pub const MAX_LENGTH: usize = 128;

pub fn check_new_password(password: &str, email: &str) -> Result<(), DomainError> {
    // Counted in characters, not bytes, so non-ASCII passwords are not penalized.
    let length = password.chars().count();
    if length < MIN_LENGTH {
        return Err(DomainError::Validation(format!(
            "the password must be at least {MIN_LENGTH} characters"
        )));
    }
    if length > MAX_LENGTH {
        return Err(DomainError::Validation(format!(
            "the password must be at most {MAX_LENGTH} characters"
        )));
    }
    let lowered = password.trim().to_lowercase();
    let email = email.trim().to_lowercase();
    let local_part = email.split('@').next().unwrap_or_default();
    if lowered == email || lowered == local_part {
        return Err(DomainError::Validation(
            "the password must not be your email address".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_is_counted_in_characters_and_bounded() {
        assert!(check_new_password("short-pass1", "a@x.com").is_err());
        assert!(check_new_password("twelve-chars", "a@x.com").is_ok());
        assert!(
            check_new_password("ççççççççççç", "a@x.com").is_err(),
            "11 chars"
        );
        assert!(
            check_new_password("çççççççççççç", "a@x.com").is_ok(),
            "12 chars"
        );
        assert!(check_new_password(&"x".repeat(MAX_LENGTH), "a@x.com").is_ok());
        assert!(check_new_password(&"x".repeat(MAX_LENGTH + 1), "a@x.com").is_err());
    }

    #[test]
    fn the_email_is_not_a_password() {
        assert!(check_new_password("Ops.Lead@Acme.com", "ops.lead@acme.com").is_err());
        assert!(check_new_password("averylongname", "averylongname@acme.com").is_err());
    }
}
