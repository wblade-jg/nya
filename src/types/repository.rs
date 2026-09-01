#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Repository {
    pub(crate) uris: Vec<String>,
    pub(crate) suites: Vec<String>,
    pub(crate) components: Vec<String>,
    pub(crate) signed_by: String,
}

impl Repository {
    pub(crate) fn new() -> Self {
        Repository {
            uris: Vec::new(),
            suites: Vec::new(),
            components: Vec::new(),
            signed_by: String::default(),
        }
    }

    pub(crate) fn component(&self) -> Option<String> {
        self.components.first().cloned()
    }

    pub(crate) fn signed_by(&self) -> String {
        self.signed_by.clone()
    }

    fn urls(&self) -> impl Iterator<Item = &String> {
        self.uris.iter()
    }

    pub(crate) fn suite(&self) -> Option<String> {
        self.suites.first().cloned()
    }

    pub fn inrelease_path(&self) -> Option<String> {
        let base_url = self.urls().next()?.clone();
        let suite = self.suite()?;
        Some(format!("{}/dists/{}/InRelease", base_url, suite))
    }

    pub fn packages_path(&self, architecture: &str) -> Option<String> {
        self.component()
            .map(|component| format!("{}/binary-{}/Packages", component, architecture))
    }

    pub(crate) fn is_valid(&self) -> bool {
        !self.uris.is_empty() && !self.suites.is_empty() && !self.components.is_empty()
    }
}
