#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Repository {
    uris: Vec<String>,
    suites: Vec<String>,
    components: Vec<String>,
    signed_by: String,
}

impl Repository {
    pub(crate) fn new(
        uris: Vec<String>,
        suites: Vec<String>,
        components: Vec<String>,
        signed_by: String,
    ) -> Result<Self, &'static str> {
        if uris.is_empty() {
            return Err("URIs");
        }
        if suites.is_empty() {
            return Err("Suites");
        }
        if components.is_empty() {
            return Err("Components");
        }
        if signed_by.is_empty() {
            return Err("Signed-By");
        }
        Ok(Repository {
            uris,
            suites,
            components,
            signed_by,
        })
    }

    pub(crate) fn component(&self) -> &str {
        self.components
            .first()
            .expect("invariante: new() exige components no vacío")
            .as_str()
    }

    pub(crate) fn signed_by(&self) -> &str {
        &self.signed_by
    }

    pub fn urls(&self) -> impl Iterator<Item = &String> {
        self.uris.iter()
    }

    #[cfg(test)]
    pub(crate) fn uris(&self) -> &[String] {
        &self.uris
    }

    #[cfg(test)]
    pub(crate) fn suites(&self) -> &[String] {
        &self.suites
    }

    #[cfg(test)]
    pub(crate) fn components(&self) -> &[String] {
        &self.components
    }

    pub(crate) fn suite(&self) -> &str {
        self.suites
            .first()
            .expect("invariante: new() exige suites no vacío")
            .as_str()
    }
}
