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

    #[test]
    fn calculate_file_hash_matches_fixture_inrelease() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/fixtures/InRelease");
        let hash = calculate_file_hash(path).unwrap();
        assert_eq!(
            hash, "902be8d8c05c4a7998bed6a2055c1be36d7ddcc7f116ef1296f4ab671cc9eb03",
            "el SHA256 del fixture InRelease debe coincadir"
        );
    }

    #[test]
    fn calculate_file_hash_missing_file_names_the_path() {
        let err = calculate_file_hash("/no/existe/fichero").unwrap_err();
        assert!(
            err.to_string().contains("/no/existe/fichero"),
            "el error debe nombrar el fichero que falta: {err}"
        );
    }

    #[test]
    fn calculate_file_hash_handles_multiple_chunks() {
        // Fichero >8192 bytes para ejercitar el loop multi-chunk.
        let path = std::env::temp_dir().join(format!("nya-hash-test-{}", std::process::id()));
        std::fs::write(&path, vec![b'a'; 20_000]).unwrap();

        let hash = calculate_file_hash(path.to_str().unwrap());
        std::fs::remove_file(&path).ok();

        assert_eq!(
            hash.unwrap(),
            "cc17faaad36649c4603dda4d8ff97cb149722af0bcac0746305a2134ad2d0b97",
            "el hash de 20000 bytes debe coincidir"
        );
    }

    #[test]
    fn validate_signature_file_missing_signed_file_names_the_path() {
        let key_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/fixtures/key.pub.asc");
        let err = validate_signature_file("/no/existe/InRelease", key_path).unwrap_err();

        assert!(
            err.to_string().contains("/no/existe/InRelease"),
            "el error debe nombrar el fichero firmado que falta: {err}"
        );
    }

    #[test]
    fn validate_signature_file_corrupt_key_errors() {
        let path = std::env::temp_dir().join(format!("nya-corrupt-key-{}", std::process::id()));
        std::fs::write(&path, "esto no es una clave PGP").unwrap();

        let err = validate_signature_file("no-importa", path.to_str().unwrap()).unwrap_err();
        std::fs::remove_file(&path).ok();

        assert!(
            err.to_string().contains("clave pública"),
            "el error debe indicar que la clave no se pudo leer: {err}"
        );
    }

    #[test]
    fn validate_signature_file_plaintext_input_errors() {
        let key_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/fixtures/key.pub.asc");
        let err = validate_signature_file(
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/fixtures/sources"),
            key_path,
        )
        .unwrap_err();

        assert!(
            err.to_string().contains("cleartext"),
            "un fichero sin cleartext PGP debe producir un error de parseo: {err}"
        );
    }
}
