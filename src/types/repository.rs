#[derive(Debug, PartialEq, Clone)]
pub struct Repository {
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

    pub fn component(&self) -> String {
        self.components
            .first()
            .expect("Se espera al menos un componente")
            .clone()
    }

    pub fn signed_by(&self) -> String {
        self.signed_by.clone()
    }

    fn urls(&self) -> impl Iterator<Item = &String> {
        self.uris.iter()
    }

    pub fn suite(&self) -> String {
        self.suites
            .first()
            .expect("Se espera al menos una suite")
            .clone()
    }

    pub fn inrelease_path(&self) -> String {
        let base_url = self.urls().next().unwrap().clone();
        let suite = self.suite();
        format!("{}/dists/{}/InRelease", base_url, suite)
    }

    pub fn packages_path(&self, architecture: &str) -> String {
        format!(
            "{}/binary-{}/Packages",
            self.component(),
            architecture
        )
    }

    pub(crate) fn is_valid(&self) -> bool {
        !self.uris.is_empty() && !self.suites.is_empty() && !self.components.is_empty()
    }
}
