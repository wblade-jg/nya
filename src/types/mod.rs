mod repository;
pub(crate) use repository::Repository;

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) enum HashType {
    MD5,
    SHA1,
    SHA256,
    SHA512,
}

impl HashType {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            HashType::MD5 => "MD5",
            HashType::SHA1 => "SHA1",
            HashType::SHA256 => "SHA256",
            HashType::SHA512 => "SHA512",
        }
    }

    #[allow(dead_code)]
    pub(crate) fn size(&self) -> usize {
        match self {
            HashType::MD5 => 32,
            HashType::SHA1 => 40,
            HashType::SHA256 => 64,
            HashType::SHA512 => 128,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct PackageFileInfo {
    hash: String,
    size: usize,
    path: String,
}

#[allow(dead_code)]
impl PackageFileInfo {
    pub(crate) fn new(hash: String, size: usize, path: String) -> Self {
        PackageFileInfo { hash, size, path }
    }

    pub(crate) fn hash(&self) -> &str {
        &self.hash
    }

    pub(crate) fn size(&self) -> usize {
        self.size
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DownloadedInRelease {
    inrelease_path: String,
    pub repository: Repository,
}

impl DownloadedInRelease {
    pub(crate) fn new(inrelease_path: String, repository: Repository) -> Self {
        DownloadedInRelease {
            inrelease_path,
            repository,
        }
    }

    pub(crate) fn inrelease_path(&self) -> &str {
        &self.inrelease_path
    }

    pub(crate) fn signature_path(&self) -> &str {
        self.repository.signed_by()
    }

    pub(crate) fn target_package_path(&self, architecture: &str) -> String {
        format!(
            "{}/binary-{}/Packages",
            self.repository.component(),
            architecture
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_ok() -> Repository {
        Repository::new(
            vec!["http://mirror.example.com/ubuntu".to_string()],
            vec!["noble".to_string()],
            vec!["main".to_string()],
            "/usr/share/keyrings/test.gpg".to_string(),
        )
        .expect("repo con todos los campos debe construirse")
    }

    #[test]
    fn target_package_path_joins_component_and_architecture() {
        let downloaded = DownloadedInRelease::new("/tmp/InRelease".to_string(), repo_ok());
        assert_eq!(
            downloaded.target_package_path("amd64"),
            "main/binary-amd64/Packages"
        );
    }

    #[test]
    fn repository_new_rejects_empty_fields() {
        let uris = vec!["http://mirror.example.com/ubuntu".to_string()];
        let suites = vec!["noble".to_string()];
        let components = vec!["main".to_string()];
        let signed_by = "/usr/share/keyrings/test.gpg".to_string();

        assert_eq!(
            Repository::new(
                Vec::new(),
                suites.clone(),
                components.clone(),
                signed_by.clone()
            ),
            Err("URIs")
        );
        assert_eq!(
            Repository::new(
                uris.clone(),
                Vec::new(),
                components.clone(),
                signed_by.clone()
            ),
            Err("Suites")
        );
        assert_eq!(
            Repository::new(uris.clone(), suites.clone(), Vec::new(), signed_by.clone()),
            Err("Components")
        );
        assert_eq!(
            Repository::new(
                uris.clone(),
                suites.clone(),
                components.clone(),
                String::new()
            ),
            Err("Signed-By")
        );
        assert!(
            Repository::new(uris, suites, components, signed_by).is_ok(),
            "todos los campos presentes debe construir"
        );
    }

    #[test]
    fn repository_accessors_return_first_values() {
        let repo = repo_ok();
        assert_eq!(repo.suite(), "noble");
        assert_eq!(repo.component(), "main");
        assert_eq!(repo.signed_by(), "/usr/share/keyrings/test.gpg");
        assert_eq!(repo.uris().len(), 1);
        assert_eq!(repo.urls().count(), 1);
    }
}
