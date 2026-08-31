use std::error::Error;
use std::fs::File;

use pgp::composed::{CleartextSignedMessage, Deserializable, SignedPublicKey};

pub fn verify_inrelease_signature(
    msg: &CleartextSignedMessage,
    key: &SignedPublicKey,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if msg.verify(key).is_ok() {
        return Ok(());
    }

    let subkey_ok = key
        .public_subkeys
        .iter()
        .any(|subkey| msg.verify(subkey).is_ok());

    if subkey_ok {
        Ok(())
    } else {
        Err("No matching signature found".into())
    }
}

pub fn validate_signature_file(
    signed_path: &str,
    key_path: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let key_file = File::open(key_path)?;
    let (public_key, _) = SignedPublicKey::from_reader_single(key_file)?;

    let signed_file = std::fs::read_to_string(signed_path)?;
    let (message, _) = CleartextSignedMessage::from_string(&signed_file)?;

    verify_inrelease_signature(&message, &public_key)?;

    Ok(message.signed_text())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_inrelease_signed_by_subkey() {
        let (key, _) = SignedPublicKey::from_string(
            include_str!("../tests/fixtures/key.pub.asc"),
        )
        .unwrap();
        let (msg, _) = CleartextSignedMessage::from_string(
            include_str!("../tests/fixtures/InRelease"),
        )
        .unwrap();

        let res = verify_inrelease_signature(&msg, &key);
        assert!(res.is_ok(), "la firma de la subclave debería validar: {res:?}");
    }

    #[test]
    fn rejects_tampered_inrelease() {
        let (key, _) = SignedPublicKey::from_string(
            include_str!("../tests/fixtures/key.pub.asc"),
        )
        .unwrap();
        let tampered = include_str!("../tests/fixtures/InRelease")
            .replace("Origin: Nya Test", "Origin: HACKER");
        let (msg, _) = CleartextSignedMessage::from_string(&tampered).unwrap();

        assert!(verify_inrelease_signature(&msg, &key).is_err());
    }
}
