use pgp::composed::{CleartextSignedMessage, Deserializable, SignedPublicKey};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs::File;
use std::io::Read;

pub(crate) fn calculate_file_hash(
    file_path: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut file =
        File::open(file_path).map_err(|e| format!("No se pudo abrir {file_path}: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| format!("No se pudo leer {file_path}: {e}"))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    let hex_string = result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    Ok(hex_string)
}

pub(crate) fn verify_inrelease_signature(
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
        Err("La firma no coincide con la clave pública".into())
    }
}

pub(crate) fn validate_signature_file(
    signed_path: &str,
    key_path: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let key_file = File::open(key_path)
        .map_err(|e| format!("No se pudo abrir la clave pública {key_path}: {e}"))?;
    let (public_key, _) = SignedPublicKey::from_reader_single(key_file)
        .map_err(|e| format!("No se pudo leer la clave pública {key_path}: {e}"))?;

    let signed_file = std::fs::read_to_string(signed_path)
        .map_err(|e| format!("No se pudo leer el fichero firmado {signed_path}: {e}"))?;
    let (message, _) = CleartextSignedMessage::from_string(&signed_file)
        .map_err(|e| format!("No se pudo parsear el cleartext de {signed_path}: {e}"))?;

    verify_inrelease_signature(&message, &public_key)
        .map_err(|_| format!("La firma no coincide con la clave {key_path}"))?;

    Ok(message.signed_text())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_inrelease_signed_by_subkey() {
        let (key, _) = SignedPublicKey::from_string(include_str!("fixtures/key.pub.asc")).unwrap();
        let (msg, _) =
            CleartextSignedMessage::from_string(include_str!("fixtures/InRelease")).unwrap();

        let res = verify_inrelease_signature(&msg, &key);
        assert!(
            res.is_ok(),
            "la firma de la subclave debería validar: {res:?}"
        );
    }

    #[test]
    fn rejects_tampered_inrelease() {
        let (key, _) = SignedPublicKey::from_string(include_str!("fixtures/key.pub.asc")).unwrap();
        let tampered =
            include_str!("fixtures/InRelease").replace("Origin: Nya Test", "Origin: HACKER");
        let (msg, _) = CleartextSignedMessage::from_string(&tampered).unwrap();

        assert!(verify_inrelease_signature(&msg, &key).is_err());
    }

    #[test]
    fn missing_key_file_error_names_the_path() {
        let err = validate_signature_file("no-importa", "/no/existe/clave.asc").unwrap_err();

        assert!(
            err.to_string().contains("/no/existe/clave.asc"),
            "el error debe nombrar la clave que falta: {err}"
        );
    }
}
