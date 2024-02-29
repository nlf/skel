use core::cmp::Ordering;
use std::fs;
use std::io;
use std::path::PathBuf;

use feruca::Collator;

use crate::error::SkelError;

pub fn read_to_string_with_default(path: &PathBuf) -> Result<(String, bool), SkelError> {
    let mut config_content = "".to_owned();
    let is_default = match fs::read_to_string(path) {
        Ok(content) => {
            config_content = content;
            false
        },
        Err(err) => {
            if err.kind() != io::ErrorKind::NotFound {
                return Err(err.into());
            }

            true
        }
    };

    Ok((config_content, is_default))
}

pub fn read_tree(dir: &PathBuf, root: &PathBuf) -> Result<Vec<PathBuf>, SkelError> {
    let mut result: Vec<PathBuf> = Vec::new();

    let dir_entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => {
            return Ok(result);
        },
    };

    for entry in dir_entries {
        let entry = entry?;
        let path = entry.path();

        // skip hidden files and node_modules
        // TODO: respect gitignore?
        let file_name = path.file_name().unwrap().to_string_lossy();
        if file_name.starts_with('.') || file_name == "node_modules" {
            continue;
        }

        if path.is_dir() {
            let child_contents = read_tree(&path, root)?;
            result.extend(child_contents);
        } else if path.is_file() {
            result.push(path.strip_prefix(root).unwrap().into());
        }
    }

    let mut collator = Collator::default();
    result.sort_by(|a, b| {
        let parent_a = a.parent().unwrap().to_str().unwrap();
        let file_name_a = a.file_name().unwrap().to_str().unwrap();

        let parent_b = b.parent().unwrap().to_str().unwrap();
        let file_name_b = b.file_name().unwrap().to_str().unwrap();

        let parent_cmp = collator.collate(parent_a, parent_b);

        match parent_cmp {
            Ordering::Equal => collator.collate(file_name_a, file_name_b),
            _ => parent_cmp,
        }
    });
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod read_to_string_with_default_helper {
        use super::*;
        use std::io::Write;
        use tempfile::NamedTempFile;

        #[test]
        fn reads_files() {
            let mut file = NamedTempFile::new().unwrap();
            write!(file, "hello").unwrap();

            let result = read_to_string_with_default(&file.path().to_path_buf());
            assert!(result.is_ok());

            let (value, is_default) = result.unwrap();
            assert_eq!(value, "hello");
            assert_eq!(is_default, false);
        }

        #[test]
        fn default_when_file_is_missing() {
            let result = read_to_string_with_default(&PathBuf::from("/A/PATH/THAT/DOES/NOT/EXIST"));
            assert!(result.is_ok());

            let (value, is_default) = result.unwrap();
            assert_eq!(value, "");
            assert_eq!(is_default, true);
        }

        #[test]
        fn surfaces_io_errors() {
            let result = read_to_string_with_default(&PathBuf::from("/"));
            assert!(result.is_err());

            let is_io_error = match result.unwrap_err() {
                SkelError::IoError(_) => true,
                _ => false,
            };
            assert!(is_io_error);
        }
    }

    mod read_tree {
        use super::*;
        use tempfile::TempDir;

        #[test]
        fn returns_relative_paths_recursively() {
            let root = TempDir::new().unwrap();
            fs::write(root.path().join("one.txt"), "should exist").unwrap();

            fs::create_dir(root.path().join(".hidden")).unwrap();
            fs::write(root.path().join(".hidden/file.txt"), "should not exist").unwrap();

            fs::create_dir(root.path().join("subdirectory")).unwrap();
            fs::write(root.path().join("subdirectory/two.txt"), "should exist").unwrap();
            fs::write(root.path().join("subdirectory/three.txt"), "should exist").unwrap();

            fs::create_dir(root.path().join("subdirectory/node_modules")).unwrap();
            fs::write(root.path().join("subdirectory/node_modules/file.txt"), "should not exist").unwrap();

            fs::create_dir(root.path().join("subdirectory/subsubdirectory")).unwrap();
            fs::write(root.path().join("subdirectory/subsubdirectory/four.txt"), "should exist").unwrap();
            
            let tree = read_tree(&root.path().to_path_buf(), &root.path().to_path_buf()).unwrap();
            assert_eq!(tree, vec![
                PathBuf::from("one.txt"),
                PathBuf::from("subdirectory/three.txt"),
                PathBuf::from("subdirectory/two.txt"),
                PathBuf::from("subdirectory/subsubdirectory/four.txt"),
            ]);
        }
    }
}
