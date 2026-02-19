#[cfg(feature = "noir-compiler")]
use std::path::Path;

#[cfg(feature = "noir-compiler")]
use crate::diagnostics::normalize_file_path;
#[cfg(feature = "noir-compiler")]
use crate::model::Span;

#[cfg(feature = "noir-compiler")]
use fm::{FileId, FileManager};
#[cfg(feature = "noir-compiler")]
use noirc_errors::Location;

#[cfg(feature = "noir-compiler")]
pub struct SpanMapper<'a> {
    root: &'a Path,
    file_manager: &'a FileManager,
}

#[cfg(feature = "noir-compiler")]
impl<'a> SpanMapper<'a> {
    pub fn new(root: &'a Path, file_manager: &'a FileManager) -> Self {
        Self { root, file_manager }
    }

    pub fn map_location(&self, location: Location) -> Span {
        let file = self.normalize_file_path(location.file);
        let start = location.span.start();
        let end = location.span.end();
        let (line, col) = self
            .file_manager
            .fetch_file(location.file)
            .map(|source| line_col_for_offset(source, usize::try_from(start).unwrap_or(0)))
            .unwrap_or((1, 1));

        Span::new(file, start, end, line, col)
    }

    pub fn is_user_file(&self, file_id: FileId) -> bool {
        self.file_manager
            .path(file_id)
            .map(|path| path.starts_with(self.root))
            .unwrap_or(false)
    }

    pub fn normalize_file_path(&self, file_id: FileId) -> String {
        match self.file_manager.path(file_id) {
            Some(path) => {
                let display_path = path
                    .strip_prefix(self.root)
                    .map(|relative| relative.to_path_buf())
                    .unwrap_or_else(|_| path.to_path_buf());
                normalize_file_path(&display_path.display().to_string())
            }
            None => format!("<unknown:{}>", file_id.as_usize()),
        }
    }
}

#[cfg(feature = "noir-compiler")]
fn line_col_for_offset(source: &str, offset: usize) -> (u32, u32) {
    let bounded = std::cmp::min(offset, source.len());
    let prefix = &source[..bounded];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let col = bounded.saturating_sub(line_start) + 1;

    (
        u32::try_from(line).unwrap_or(u32::MAX),
        u32::try_from(col).unwrap_or(u32::MAX),
    )
}

#[cfg(test)]
#[cfg(feature = "noir-compiler")]
mod tests {
    use std::path::Path;

    use fm::FileManager;
    use noirc_errors::{Location, Span as NoirSpan};

    use super::SpanMapper;

    #[test]
    fn maps_line_and_column_from_byte_offset() {
        let root = Path::new(".");
        let mut files = FileManager::new(root);
        let source = "fn main() {\n let x = 42;\n}\n";
        let file_id = files
            .add_file_with_source(Path::new("src/main.nr"), source.to_string())
            .expect("file should be added");

        let mapper = SpanMapper::new(root, &files);
        let start = u32::try_from(source.find("let").expect("marker exists")).expect("fits in u32");
        let location = Location::new(NoirSpan::from(start..(start + 3)), file_id);
        let span = mapper.map_location(location);

        assert_eq!(span.line, 2);
        assert_eq!(span.col, 2);
        assert_eq!(span.start, start);
        assert_eq!(span.end, start + 3);
    }
}
