use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Content {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub kind: ContentKind,
    pub dependencies: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentKind {
    File,
    Template,
}

const STR_FILE: &str = "file";
const STR_TEMPLATE: &str = "template";

impl ContentKind {
    fn from_str_opt(input: Option<&str>) -> ContentKind {
        match input {
            Some(input) => match input.trim().to_lowercase().as_ref() {
                STR_FILE => ContentKind::File,
                STR_TEMPLATE => ContentKind::Template,
                _ => panic!("invalid content kind: {}", &input),
            },
            None => ContentKind::File,
        }
    }
}

impl Content {
    pub fn from_source(path: &Path, kind: Option<&str>) -> Self {
        let source = path.to_path_buf();
        let mut destination = PathBuf::from(source.parent().unwrap());
        let file_name: String = source.file_name().unwrap().to_string_lossy().into();
        if file_name.starts_with("dot_") {
            destination.push(format!(".{}", file_name.strip_prefix("dot_").unwrap()));
        } else {
            destination.push(file_name);
        }

        Self {
            source,
            destination,
            kind: ContentKind::from_str_opt(kind),
            dependencies: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Content, ContentKind};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn from_source() {
        let root = TempDir::new().unwrap();
        let full_path = root.path().join("file.txt");

        fs::write(&full_path, "").unwrap();

        let content = Content::from_source(&full_path, None);
        assert_eq!(content.source, full_path);
        assert_eq!(content.destination, full_path);
        assert_eq!(content.kind, ContentKind::File);
    }

    #[test]
    fn from_source_dot_prefix() {
        let root = TempDir::new().unwrap();
        let full_path = root.path().join("dot_file.txt");

        fs::write(&full_path, "").unwrap();

        let content = Content::from_source(&full_path, None);
        assert_eq!(content.source, full_path);
        assert_eq!(content.destination, root.path().join(".file.txt"));
        assert_eq!(content.kind, ContentKind::File);
    }

    #[test]
    fn from_source_kind_file() {
        let root = TempDir::new().unwrap();
        let full_path = root.path().join("file.txt");

        fs::write(&full_path, "").unwrap();

        let content = Content::from_source(&full_path, Some("file"));
        assert_eq!(content.source, full_path);
        assert_eq!(content.destination, full_path);
        assert_eq!(content.kind, ContentKind::File);
    }

    #[test]
    fn from_source_kind_template() {
        let root = TempDir::new().unwrap();
        let full_path = root.path().join("file.template");
        fs::write(&full_path, "").unwrap();

        let content = Content::from_source(&full_path, Some("template"));
        assert_eq!(content.source, full_path);
        assert_eq!(content.destination, root.path().join("file.template"));
        assert_eq!(content.kind, ContentKind::Template);
    }
}
