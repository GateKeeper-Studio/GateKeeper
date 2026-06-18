use serde_json::Value;
use uuid::Uuid;
use webauthn_rs::prelude::*;

use crate::{
    application::ports::webauthn::{
        AuthenticatedPasskey, CeremonyStart, RegisteredPasskey, WebauthnService,
    },
    domain::errors::DomainError,
};

// Production WebAuthn ceremonies via the webauthn-rs crate.
//
// The relying party identity (rp_id + origin) must match the URL the browser
// shows during the ceremony — i.e. the auth UI's host — or every registration
// and assertion fails client-side. See resolve_webauthn_rp() in config.rs.
//
// Ceremony state (PasskeyRegistration / PasskeyAuthentication) is serialized
// to JSON so the use case can persist it across the two stateless HTTP calls
// of a ceremony (the `danger-allow-state-serialisation` feature). It stays
// server-side: short-lived, single-use, never sent to clients.
pub struct WebauthnRsService {
    webauthn: Webauthn,
}

impl WebauthnRsService {
    // `additional_origins`: other first-party browser origins allowed to run
    // ceremonies (e.g. a BFF client enrolling passkeys from its own host).
    // They must all share the rp_id's effective domain.
    pub fn new(
        rp_id: &str,
        rp_origin: &Url,
        rp_name: &str,
        additional_origins: &[Url],
    ) -> Result<Self, DomainError> {
        let mut builder = WebauthnBuilder::new(rp_id, rp_origin)
            .map_err(|e| {
                DomainError::InfrastructureError(format!(
                    "invalid WebAuthn relying party config: {e}"
                ))
            })?
            .rp_name(rp_name);
        for origin in additional_origins {
            builder = builder.append_allowed_origin(origin);
        }
        let webauthn = builder.build().map_err(|e| {
            DomainError::InfrastructureError(format!("failed to build WebAuthn context: {e}"))
        })?;
        Ok(Self { webauthn })
    }
}

// All failures collapse to a generic MfaError: ceremony details are logged
// server-side but never echoed to the caller (no oracle).
fn mfa_err(context: &str) -> impl Fn(WebauthnError) -> DomainError + '_ {
    move |e| {
        tracing::warn!("webauthn {context} failed: {e}");
        DomainError::MfaError("webauthn ceremony failed".to_string())
    }
}

fn decode_err(context: &'static str) -> impl Fn(serde_json::Error) -> DomainError {
    move |e| {
        tracing::warn!("webauthn {context} decode failed: {e}");
        DomainError::MfaError("webauthn ceremony failed".to_string())
    }
}

// CredentialID is Base64UrlSafeData; its serde form is the base64url string,
// so string ↔ id conversions go through serde_json rather than guessing at
// inherent conversions.
fn credential_id_to_string(id: &CredentialID) -> Result<String, DomainError> {
    match serde_json::to_value(id) {
        Ok(Value::String(s)) => Ok(s),
        _ => Err(DomainError::MfaError(
            "webauthn ceremony failed".to_string(),
        )),
    }
}

fn credential_id_from_string(raw: &str) -> Result<CredentialID, DomainError> {
    serde_json::from_value(Value::String(raw.to_string())).map_err(decode_err("credential id"))
}

fn parse_passkeys(passkeys: &[Value]) -> Result<Vec<Passkey>, DomainError> {
    passkeys
        .iter()
        .map(|p| serde_json::from_value::<Passkey>(p.clone()).map_err(decode_err("stored passkey")))
        .collect()
}

impl WebauthnService for WebauthnRsService {
    fn start_registration(
        &self,
        user_id: Uuid,
        user_email: &str,
        user_display_name: &str,
        exclude_credential_ids: &[String],
    ) -> Result<CeremonyStart, DomainError> {
        let exclude: Option<Vec<CredentialID>> = if exclude_credential_ids.is_empty() {
            None
        } else {
            Some(
                exclude_credential_ids
                    .iter()
                    .map(|raw| credential_id_from_string(raw))
                    .collect::<Result<_, _>>()?,
            )
        };

        let (options, state) = self
            .webauthn
            .start_passkey_registration(user_id, user_email, user_display_name, exclude)
            .map_err(mfa_err("registration start"))?;

        Ok(CeremonyStart {
            options: serde_json::to_value(options).map_err(decode_err("registration options"))?,
            state: serde_json::to_value(state).map_err(decode_err("registration state"))?,
        })
    }

    fn finish_registration(
        &self,
        attestation: &Value,
        state: &Value,
    ) -> Result<RegisteredPasskey, DomainError> {
        let attestation: RegisterPublicKeyCredential =
            serde_json::from_value(attestation.clone()).map_err(decode_err("attestation"))?;
        let state: PasskeyRegistration =
            serde_json::from_value(state.clone()).map_err(decode_err("registration state"))?;

        let passkey = self
            .webauthn
            .finish_passkey_registration(&attestation, &state)
            .map_err(mfa_err("registration finish"))?;

        Ok(RegisteredPasskey {
            credential_id: credential_id_to_string(passkey.cred_id())?,
            passkey: serde_json::to_value(&passkey).map_err(decode_err("passkey"))?,
        })
    }

    fn start_authentication(&self, passkeys: &[Value]) -> Result<CeremonyStart, DomainError> {
        let passkeys = parse_passkeys(passkeys)?;

        let (options, state) = self
            .webauthn
            .start_passkey_authentication(&passkeys)
            .map_err(mfa_err("authentication start"))?;

        Ok(CeremonyStart {
            options: serde_json::to_value(options).map_err(decode_err("authentication options"))?,
            state: serde_json::to_value(state).map_err(decode_err("authentication state"))?,
        })
    }

    fn finish_authentication(
        &self,
        assertion: &Value,
        state: &Value,
        passkeys: &[Value],
    ) -> Result<AuthenticatedPasskey, DomainError> {
        let assertion: PublicKeyCredential =
            serde_json::from_value(assertion.clone()).map_err(decode_err("assertion"))?;
        let state: PasskeyAuthentication =
            serde_json::from_value(state.clone()).map_err(decode_err("authentication state"))?;

        let result = self
            .webauthn
            .finish_passkey_authentication(&assertion, &state)
            .map_err(mfa_err("authentication finish"))?;

        let credential_id = credential_id_to_string(result.cred_id())?;

        // Advance the stored signature counter when the authenticator moved it —
        // persisting this is what makes a cloned credential detectable later.
        let updated_passkey = if result.needs_update() {
            let mut stored = parse_passkeys(passkeys)?;
            stored
                .iter_mut()
                .find(|p| p.cred_id() == result.cred_id())
                .and_then(|p| p.update_credential(&result).map(|_| p.clone()))
                .map(|p| serde_json::to_value(&p).map_err(decode_err("updated passkey")))
                .transpose()?
        } else {
            None
        };

        Ok(AuthenticatedPasskey {
            credential_id,
            updated_passkey,
        })
    }
}
