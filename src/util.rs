use std::path::{Component, Path, PathBuf};
use crate::error::SkelError;

pub fn normalize_path<T>(from: T, path: T) -> Result<PathBuf, SkelError>
where
    T: AsRef<Path>,
{
    let mut components: Vec<Component> = Vec::new();
    for component in from.as_ref().components() {
        components.push(component)
    }

    for component in path.as_ref().components() {
        match component {
            Component::RootDir => {
                components.clear();
                components.push(Component::RootDir);
            }
            Component::ParentDir => {
                components.pop();
            }
            Component::Normal(dir) => {
                components.push(Component::Normal(dir));
            }
            // CurDir is "./" so they get skipped
            // Prefix is only used in Windows, which we don't care about
            _ => {},
        };
    }

    let result = components.iter().map(|c| {
        match c {
            Component::Normal(dir) => dir.to_str().unwrap().to_owned(),
            _ => "".to_owned(),
        }
    }).collect::<Vec<String>>();

    Ok(PathBuf::from(result.join("/")))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    mod normalize_path {
        use super::*;
        use std::env;

        #[test]
        fn normalizes_root_path() {
            let from = env::current_dir().unwrap();
            let path = PathBuf::from("/test");
            let result = normalize_path(from, path).unwrap();
            assert_eq!(result, PathBuf::from("/test"));
        }

        #[test]
        fn normalizes_parent_dir() {
            let from = env::current_dir().unwrap();
            let path = PathBuf::from("../test");
            let result = normalize_path(&from, &path).unwrap();
            assert_eq!(result, from.parent().unwrap().join("test"));
        }

        #[test]
        fn normalizes_current_dir() {
            let from = env::current_dir().unwrap();
            let path = PathBuf::from("./test");
            let result = normalize_path(&from, &path).unwrap();
            assert_eq!(result, from.join("test"));
        }
    }
}
