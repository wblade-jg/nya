use std::io;

fn format_causas(causas: &[String]) -> String {
    causas
        .iter()
        .map(|causa| format!("    - {causa}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, thiserror::Error)]
pub enum NyaError {
    #[error("No se pudo abrir {path}: {source}")]
    OpenFile {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("No se pudo leer {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("No se pudo guardar {path}: {source}")]
    StoreFile {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("No se encontró el fichero de repositorios {path}")]
    MissingRepositoriesFile { path: String },

    #[error("No se pudo crear el directorio de caché {path}: {source}")]
    CreateCacheDir {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("Arquitectura no soportada: {0}")]
    UnsupportedArch(String),

    #[error("{path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: crate::parser::ParseError,
    },

    #[error("La firma no coincide con la clave {key}")]
    SignatureMismatch { key: String },

    #[error("No se pudo leer la clave pública {path}: {reason}")]
    InvalidKey { path: String, reason: String },

    #[error("No se pudo parsear el cleartext de {path}: {reason}")]
    InvalidCleartext { path: String, reason: String },

    #[error("{source}")]
    Http {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("No se pudo crear el cliente HTTP: {0}")]
    HttpClient(String),

    #[error("No se encontró suite en el repositorio")]
    MissingSuite,

    #[error("No se encontró component en el repositorio")]
    MissingComponent,

    #[error("No se encontro la seccion {section} en el InRelease")]
    MissingSection { section: &'static str },

    #[error("No se encontró {path} en la sección {section}")]
    PackageEntryNotFound { section: &'static str, path: String },

    #[error("Hash invalido (longitud o caracteres) para {path}")]
    InvalidPackageHash { path: String },

    #[error("Tamaño no numerico para {path}")]
    InvalidPackageSize { path: String },

    #[error("{what} no descargable de ningún mirror:\n{}", format_causas(.causas))]
    AllMirrorsFailed {
        what: &'static str,
        causas: Vec<String>,
    },

    #[error("No hay mirrors de {what} que probar")]
    NoMirrors { what: &'static str },

    #[error("{failed} de {total} repositorios fallaron durante la actualización")]
    UpdateFailed { failed: usize, total: usize },

    #[error("Error en spawn_blocking: {0}")]
    Join(String),
}

impl NyaError {
    pub(crate) fn mirrors_failed(what: &'static str, causas: &[String]) -> Self {
        if causas.is_empty() {
            NyaError::NoMirrors { what }
        } else {
            NyaError::AllMirrorsFailed {
                what,
                causas: causas.to_vec(),
            }
        }
    }
}
