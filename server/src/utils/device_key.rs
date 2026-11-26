// Trusted-device keys: ECDSA P-256 with SHA-256 ("ES256"), the one algorithm every
// phone's secure hardware offers (iOS Secure Enclave, Android Keystore / StrongBox).
//
// The formats a mobile platform hands out are accepted as they come, so an app needs
// no conversion code:
//   * public key: X.509 SubjectPublicKeyInfo DER (Android `PublicKey.getEncoded()`),
//     or a SEC1 point (iOS `SecKeyCopyExternalRepresentation`, 65 bytes uncompressed;
//     compressed points too);
//   * signature: ASN.1 DER (Android `SHA256withECDSA`, iOS
//     `.ecdsaSignatureMessageX962SHA256`) or the raw 64-byte r‖s (WebCrypto);
//   * base64 in either alphabet, with or without padding.
// Keys are stored normalized: the uncompressed SEC1 point, base64url without padding.
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
};
use p256::{
    PublicKey,
    ecdsa::{Signature, VerifyingKey, signature::Verifier},
    elliptic_curve::sec1::ToEncodedPoint,
    pkcs8::DecodePublicKey,
};

pub const ALGORITHM: &str = "ES256";

fn decode_base64(value: &str) -> Option<Vec<u8>> {
    let value = value.trim();
    [&URL_SAFE_NO_PAD, &URL_SAFE, &STANDARD, &STANDARD_NO_PAD]
        .iter()
        .find_map(|engine| engine.decode(value).ok())
}

// The public key in its stored form, or None when it is not a P-256 public key.
pub fn normalize_public_key(encoded: &str) -> Option<String> {
    let bytes = decode_base64(encoded)?;
    let key = PublicKey::from_public_key_der(&bytes)
        .or_else(|_| PublicKey::from_sec1_bytes(&bytes))
        .ok()?;
    Some(URL_SAFE_NO_PAD.encode(key.to_encoded_point(false).as_bytes()))
}

// Whether `signature` is a valid ES256 signature of `message` by the stored key.
// Never panics: anything malformed is simply not a valid signature.
pub fn verify(stored_public_key: &str, message: &[u8], signature: &str) -> bool {
    let Some(key_bytes) = decode_base64(stored_public_key) else {
        return false;
    };
    let Ok(key) = VerifyingKey::from_sec1_bytes(&key_bytes) else {
        return false;
    };
    let Some(signature_bytes) = decode_base64(signature) else {
        return false;
    };
    let parsed =
        Signature::from_der(&signature_bytes).or_else(|_| Signature::from_slice(&signature_bytes));
    let Ok(signature) = parsed else {
        return false;
    };
    // A high-S signature is as valid mathematically; some platforms emit it.
    let signature = signature.normalize_s().unwrap_or(signature);
    key.verify(message, &signature).is_ok()
}

#[cfg(test)]
mod tests {
    use p256::{
        ecdsa::{SigningKey, signature::Signer},
        pkcs8::EncodePublicKey,
    };

    use super::*;

    fn signing_key(seed: u8) -> SigningKey {
        SigningKey::from_slice(&[seed; 32]).unwrap()
    }

    fn spki_b64(key: &SigningKey) -> String {
        STANDARD.encode(key.verifying_key().to_public_key_der().unwrap().as_bytes())
    }

    fn sec1_b64(key: &SigningKey, compressed: bool) -> String {
        URL_SAFE_NO_PAD.encode(key.verifying_key().to_encoded_point(compressed).as_bytes())
    }

    #[test]
    fn every_platform_key_format_normalizes_to_the_same_value() {
        let key = signing_key(7);
        let from_spki = normalize_public_key(&spki_b64(&key)).unwrap();
        assert_eq!(
            normalize_public_key(&sec1_b64(&key, false)).unwrap(),
            from_spki
        );
        assert_eq!(
            normalize_public_key(&sec1_b64(&key, true)).unwrap(),
            from_spki
        );
        // Standard base64 with padding of the raw point, as iOS apps often send it.
        let raw = key.verifying_key().to_encoded_point(false);
        assert_eq!(
            normalize_public_key(&STANDARD.encode(raw.as_bytes())).unwrap(),
            from_spki
        );
        assert_eq!(URL_SAFE_NO_PAD.decode(&from_spki).unwrap().len(), 65);
    }

    #[test]
    fn anything_else_is_not_a_key() {
        assert!(normalize_public_key("").is_none());
        assert!(normalize_public_key("not base64 !!").is_none());
        assert!(normalize_public_key(&URL_SAFE_NO_PAD.encode([4u8; 65])).is_none());
        assert!(normalize_public_key(&URL_SAFE_NO_PAD.encode([1u8; 32])).is_none());
    }

    #[test]
    fn der_and_raw_signatures_verify_and_bind_the_message_and_key() {
        let key = signing_key(9);
        let stored = normalize_public_key(&spki_b64(&key)).unwrap();
        let message = b"gatekeeper.qr-login.approve.v1\nabc";
        let signature: Signature = key.sign(message);

        let der = STANDARD.encode(signature.to_der().as_bytes());
        let raw = URL_SAFE_NO_PAD.encode(signature.to_bytes());
        assert!(verify(&stored, message, &der));
        assert!(verify(&stored, message, &raw));

        assert!(!verify(&stored, b"another message", &der));
        let other = normalize_public_key(&spki_b64(&signing_key(10))).unwrap();
        assert!(!verify(&other, message, &der));
        assert!(!verify(&stored, message, "garbage"));
        assert!(!verify("garbage", message, &der));
    }
}
