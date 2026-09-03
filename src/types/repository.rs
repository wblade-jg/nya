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

    pub(crate) fn component(&self) -> Option<&str> {
        self.components.first().map(|s| s.as_str())
    }

    pub(crate) fn signed_by(&self) -> &str {
        &self.signed_by
    }

    pub fn urls(&self) -> impl Iterator<Item = &String> {
        self.uris.iter()
    }

    pub(crate) fn suite(&self) -> Option<&str> {
        self.suites.first().map(|s| s.as_str())
    }

    pub(crate) fn is_valid(&self) -> bool {
        !self.uris.is_empty() && !self.suites.is_empty() && !self.components.is_empty()
    }
}
