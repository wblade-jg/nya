use crate::types::{HashType, PackageFileInfo};

pub(crate) fn get_package_file_info(
    content: &str,
    target_path: &str,
) -> Result<PackageFileInfo, Box<dyn std::error::Error + Send + Sync>> {
    let hash_type = HashType::SHA256;

    let block_hash_start = content
        .find(hash_type.as_str())
        .ok_or_else(|| {
            format!(
                "No se encontro la seccion {} en el InRelease",
                hash_type.as_str()
            )
        })?;

    let content_from_hash_start = &content[block_hash_start..];

    for line in content_from_hash_start.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 3 {
            continue;
        }

        let hash = parts[0].to_string();
        let size = parts[1].parse::<usize>()?;
        let path = parts[2].to_string();

        if path == target_path || path.ends_with(target_path) {
            return Ok(PackageFileInfo::new(hash, size, path));
        }
    }

    Err(format!(
        "No se encontró {} en la sección {}",
        target_path,
        hash_type.as_str()
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_package_file_info() {
        let hash_type = HashType::SHA256;
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
        let result = get_package_file_info(inrelease_content, target_path);
        assert!(result.is_ok(), "get_package_file_info should return Ok");
        let package_file_info = result.unwrap();
        assert_eq!(
            package_file_info.hash().len(),
            hash_type.size(),
            "hash should be {} characters long",
            hash_type.size()
        );
    }
}
