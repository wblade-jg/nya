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
        format!("{}/binary-{}/Packages", self.repository.component().unwrap(), architecture)
    }
}
