use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub mod call_graph;
pub mod driver;
pub mod project_builder;
pub mod semantic_builder;
pub mod span_mapper;

pub use driver::NoirCheckedProject;
pub use project_builder::{
    ProjectSemanticBundle, build_project_model, build_project_semantic_bundle,
};

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
        count: usize,
    },
    CheckDiagnostics {
        count: usize,
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
            Self::ParserDiagnostics { count } => {
                write!(
                    f,
                    "Noir parser reported {count} issue(s). See diagnostics above."
                )
            }
            Self::CheckDiagnostics { count } => {
                write!(
                    f,
                    "Noir semantic checks reported {count} issue(s). See diagnostics above."
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

#[cfg(test)]
mod tests {
    use super::NoirFrontendError;

    #[test]
    fn semantic_error_display_references_reported_diagnostics() {
        let err = NoirFrontendError::CheckDiagnostics { count: 2 };

        let rendered = err.to_string();
        assert!(rendered.contains("2 issue(s)"));
        assert!(rendered.contains("See diagnostics above"));
    }
}
