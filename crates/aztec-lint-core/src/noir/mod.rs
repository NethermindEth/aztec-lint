use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub mod call_graph;
pub mod driver;
pub mod project_builder;
pub mod span_mapper;

pub use driver::NoirCheckedProject;
pub use project_builder::build_project_model;

#[derive(Debug)]
pub enum NoirFrontendError {
    CompilerFeatureDisabled,
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    EntryFileMissing {
        entry: PathBuf,
    },
    ParserDiagnostics {
        messages: Vec<String>,
    },
    CheckDiagnostics {
        messages: Vec<String>,
    },
    Internal(String),
}

impl Display for NoirFrontendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompilerFeatureDisabled => {
                write!(
                    f,
                    "Noir compiler integration is disabled (enable `noir-compiler` feature)"
                )
            }
            Self::Io { path, source } => {
                write!(
                    f,
                    "failed to read noir source '{}': {source}",
                    path.display()
                )
            }
            Self::EntryFileMissing { entry } => {
                write!(
                    f,
                    "entry noir file '{}' is missing from project sources",
                    entry.display()
                )
            }
            Self::ParserDiagnostics { messages } => {
                write!(f, "Noir parser reported {} issue(s)", messages.len())
            }
            Self::CheckDiagnostics { messages } => {
                write!(
                    f,
                    "Noir semantic checks reported {} issue(s)",
                    messages.len()
                )
            }
            Self::Internal(message) => write!(f, "internal noir frontend error: {message}"),
        }
    }
}

impl Error for NoirFrontendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::CompilerFeatureDisabled
            | Self::EntryFileMissing { .. }
            | Self::ParserDiagnostics { .. }
            | Self::CheckDiagnostics { .. }
            | Self::Internal(_) => None,
        }
    }
}
