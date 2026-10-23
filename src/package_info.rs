use crate::error::NyaError;
use crate::types::{HashType, PackageFileInfo};

pub(crate) fn get_package_file_info(
    content: &str,
    target_path: &str,
) -> Result<PackageFileInfo, NyaError> {
    let hash_type = HashType::SHA256;
    let header = hash_type.as_str();
    let header_tag = format!("{}:", header);

    let lines: Vec<&str> = content.lines().collect();

    let header_idx = lines
        .iter()
        .position(|line| line.trim_start().starts_with(&header_tag))
        .ok_or(NyaError::MissingSection { section: header })?;

    let header_line = lines[header_idx];
    let header_indent = header_line.len() - header_line.trim_start().len();

    for &line in &lines[header_idx + 1..] {
        if line.trim().is_empty() {
            continue;
        }

        let indent = line.len() - line.trim_start().len();
        if indent <= header_indent {
            break;
        }

        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 3 {
            continue;
        }

        let (hash, size_str, path) = (parts[0], parts[1], parts[2]);
        if path != target_path {
            continue;
        }

        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(NyaError::InvalidPackageHash {
                path: target_path.to_string(),
            });
        }

        let size = size_str
            .parse::<usize>()
            .map_err(|_| NyaError::InvalidPackageSize {
                path: target_path.to_string(),
            })?;

        return Ok(PackageFileInfo::new(
            hash.to_string(),
            size,
            path.to_string(),
        ));
    }

    Err(NyaError::PackageEntryNotFound {
        section: header,
        path: target_path.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_package_file_info() {
        let inrelease_content = r#"
          Origin: Test
          Suite: stable
          
          SHA256:
           a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2 12345 main/binary-amd64/Packages
           f1e2d3c4b5a697887766554433221100ffeeddccbbaa99887766554433221100 67890 main/binary-amd64/Packages.gz
          
          MD5Sum:
           aaaa 12345 main/binary-amd64/Packages
          "#;
        let target_path = "main/binary-amd64/Packages";
        let package_file_info = get_package_file_info(inrelease_content, target_path).unwrap();
        assert_eq!(
            package_file_info.hash(),
            "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2"
        );
        assert_eq!(package_file_info.size(), 12345);
        assert_eq!(package_file_info.path(), target_path);
    }

    #[test]
    fn entry_only_in_md5sum_section_returns_err() {
        let inrelease_content = r#"
          Origin: Test
          Suite: stable
          
          SHA256:
           aaaa 12345 main/binary-amd64/Other
          
          MD5Sum:
           a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6 12345 main/binary-amd64/Packages
          "#;
        let target_path = "main/binary-amd64/Packages";
        let err = get_package_file_info(inrelease_content, target_path).unwrap_err();
        assert!(
            matches!(&err, NyaError::PackageEntryNotFound { section, path } if *section == "SHA256" && path == target_path),
            "solo debe buscar en SHA256: {err:?}"
        );
    }

    #[test]
    fn non_target_line_is_skipped_regardless_of_its_fields() {
        let inrelease_content = r#"
          Origin: Test
          Suite: stable
          
          SHA256:
           a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2 abc main/binary-amd64/Bad
           f1e2d3c4b5a697887766554433221100ffeeddccbbaa99887766554433221100 67890 main/binary-amd64/Packages
          "#;
        let target_path = "main/binary-amd64/Packages";
        let result = get_package_file_info(inrelease_content, target_path);
        assert!(
            result.is_ok(),
            "una línea cuyo path no coincide se salta sin validar sus campos: {result:?}"
        );
    }

    #[test]
    fn path_prefix_mismatch_does_not_match() {
        let inrelease_content = r#"
          Origin: Test
          Suite: stable
          
          SHA256:
           a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2 12345 notmain/binary-amd64/Packages
          "#;
        let target_path = "main/binary-amd64/Packages";
        let err = get_package_file_info(inrelease_content, target_path).unwrap_err();
        assert!(
            matches!(&err, NyaError::PackageEntryNotFound { path, .. } if path == target_path),
            "notmain/... no debe coincidir con main/...: {err:?}"
        );
    }

    #[test]
    fn invalid_hash_length_or_hex_returns_err() {
        let inrelease_content_short = r#"
          Origin: Test
          Suite: stable
          
          SHA256:
           a1b2c3 12345 main/binary-amd64/Packages
          "#;
        let err = get_package_file_info(inrelease_content_short, "main/binary-amd64/Packages")
            .unwrap_err();
        assert!(
            matches!(&err, NyaError::InvalidPackageHash { path } if path == "main/binary-amd64/Packages"),
            "hash de longitud inválida: {err:?}"
        );

        let inrelease_content_nonhex = r#"
          Origin: Test
          Suite: stable
          
          SHA256:
           zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz 12345 main/binary-amd64/Packages
          "#;
        let err = get_package_file_info(inrelease_content_nonhex, "main/binary-amd64/Packages")
            .unwrap_err();
        assert!(
            matches!(&err, NyaError::InvalidPackageHash { .. }),
            "hash no-hex: {err:?}"
        );
    }

    #[test]
    fn missing_sha256_section_returns_err() {
        let inrelease_content = r#"
          Origin: Test
          Suite: stable
          
          MD5Sum:
           a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6 12345 main/binary-amd64/Packages
          "#;
        let err =
            get_package_file_info(inrelease_content, "main/binary-amd64/Packages").unwrap_err();
        assert!(
            matches!(&err, NyaError::MissingSection { section } if *section == "SHA256"),
            "sin sección SHA256: {err:?}"
        );
    }

    #[test]
    fn non_numeric_size_on_target_line_returns_err() {
        let inrelease_content = r#"
          Origin: Test
          Suite: stable
          
          SHA256:
           a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2 abc main/binary-amd64/Packages
          "#;
        let err =
            get_package_file_info(inrelease_content, "main/binary-amd64/Packages").unwrap_err();
        assert!(
            matches!(&err, NyaError::InvalidPackageSize { path } if path == "main/binary-amd64/Packages"),
            "tamaño no numérico en la línea objetivo: {err:?}"
        );
    }

    #[test]
    fn uppercase_hash_is_accepted() {
        // Documenta el comportamiento actual: el parser acepta hash en
        // mayúsculas, pero calculate_file_hash produce minúsculas; la
        // comparación en update daría falso "hash no coincide" en cada
        // ejecución si un InRelease publicara hashes en mayúsculas.
        let inrelease_content = r#"
          Origin: Test
          Suite: stable
          
          SHA256:
           A1B2C3D4E5F6A7B8C9D0E1F2A3B4C5D6E7F8A9B0C1D2E3F4A5B6C7D8E9F0A1B2 12345 main/binary-amd64/Packages
          "#;
        let result = get_package_file_info(inrelease_content, "main/binary-amd64/Packages");
        assert!(
            result.is_ok(),
            "un hash en mayúsculas debe aceptarse (comportamiento actual): {result:?}"
        );
    }
}
