use std::io;

fn format_causes(causes: &[String]) -> String {
    causes
        .iter()
        .map(|cause| format!("    - {cause}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, thiserror::Error)]
pub enum NyaError {
    #[error("No se pudo {op} {path}: {source}")]
    Io {
        op: &'static str,
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("No se encontró el fichero de repositorios {path}")]
    MissingRepositoriesFile { path: String },

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
        #[source]
        source: reqwest::Error,
    },

    #[error("No se encontro la seccion {section} en el InRelease")]
    MissingSection { section: &'static str },

    #[error("No se encontró {path} en la sección {section}")]
    PackageEntryNotFound { section: &'static str, path: String },

    #[error("Hash invalido (longitud o caracteres) para {path}")]
    InvalidPackageHash { path: String },

    #[error("Tamaño no numerico para {path}")]
    InvalidPackageSize { path: String },

    #[error("{what} no descargable de ningún mirror:\n{}", format_causes(.causes))]
    AllMirrorsFailed {
        what: &'static str,
        causes: Vec<String>,
    },

    #[error("No hay mirrors de {what} que probar")]
    NoMirrors { what: &'static str },

    #[error("{failed} de {total} repositorios fallaron durante la actualización")]
    UpdateFailed { failed: usize, total: usize },

    #[error("Error en spawn_blocking: {0}")]
    Join(String),
}

impl NyaError {
    pub(crate) fn io(op: &'static str, path: &str, source: io::Error) -> Self {
        NyaError::Io {
            op,
            path: path.to_string(),
            source,
        }
    }

    pub(crate) fn mirrors_failed(what: &'static str, causes: &[String]) -> Self {
        if causes.is_empty() {
            NyaError::NoMirrors { what }
        } else {
            NyaError::AllMirrorsFailed {
                what,
                causes: causes.to_vec(),
            }
        }
    }
}
