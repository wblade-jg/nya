mod repository;
pub use repository::Repository;

#[derive(Debug, Clone, Copy)]
pub enum HashType {
    MD5,
    SHA1,
    SHA256,
    SHA512,
}

impl HashType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HashType::MD5 => "MD5",
            HashType::SHA1 => "SHA1",
            HashType::SHA256 => "SHA256",
            HashType::SHA512 => "SHA512",
        }
    }

    pub fn size(&self) -> usize {
        match self {
            HashType::MD5 => 32,
            HashType::SHA1 => 40,
            HashType::SHA256 => 64,
            HashType::SHA512 => 128,
        }
    }
}

pub struct PackageFileInfo {
    hash: String,
    size: usize,
    path: String,
}

impl PackageFileInfo {
    pub fn new(hash: String, size: usize, path: String) -> Self {
        PackageFileInfo { hash, size, path }
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Debug, Clone)]
pub struct DownloadedInRelease {
    inrelease_path: String,
    signature_path: String,
    target_package_path: String,
}

impl DownloadedInRelease {
    pub fn new(
        inrelease_path: String,
        signature_path: String,
        target_package_path: String,
    ) -> Self {
        DownloadedInRelease {
            inrelease_path,
            signature_path,
            target_package_path,
        }
    }

    pub fn inrelease_path(&self) -> &str {
        &self.inrelease_path
    }

    pub fn signature_path(&self) -> &str {
        &self.signature_path
    }

    pub fn target_package_path(&self) -> &str {
        &self.target_package_path
    }
}
