use core::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;

use feruca::Collator;
use kdl::KdlDocument;
use tera::Context;

use crate::config::fs_helpers;
use crate::config::kdl_helpers;
use crate::config::task::Task;
use crate::content::Content;
use crate::error::{ConfigError, SkelError};

#[derive(Debug, Default)]
pub struct SkeletonConfig {
    pub root: PathBuf,
    pub content: HashMap<String, Content>,
    pub tasks: HashMap<String, Task>,
    pub variables: Context,
    pub is_default: bool,
}

impl SkeletonConfig {
    pub fn read_from(path: &PathBuf) -> Result<Self, SkelError> {
        let (config_content, is_default) = fs_helpers::read_to_string_with_default(path)?;
        let document: KdlDocument = config_content.parse()?;

        let root: PathBuf = path.parent().unwrap().join("content");

        let mut tasks: HashMap<String, Task> = HashMap::new();
        for node in document.nodes() {
            if node.name().value() != "task" {
                continue;
            }

            let name = match node.get(0) {
                Some(name) => match name.value().as_string() {
                    Some(value) => Ok(value.to_owned()),
                    None => Err(ConfigError::from_invalid_string_argument(
                        &document, node, 0,
                    )),
                },
                None => Err(ConfigError::from_missing_argument(&document, "task")),
            }?;

            if let Some(children) = node.children() {
                let task = Task::from_kdl_document(children, name.to_owned())?;
                tasks.insert(name, task);
            }
        }

        let mut content: HashMap<String, Content> = HashMap::new();
        let content_tree = fs_helpers::read_tree(&root.clone(), &root)?;
        for source in content_tree {
            content.insert(
                source.to_string_lossy().into(),
                Content::from_source(&source, None),
            );
        }

        for node in document.nodes() {
            if node.name().value() != "content" {
                continue;
            }

            let source = match node.get(0) {
                Some(entry) => match entry.value().as_string() {
                    Some(value) => Ok(value.to_owned()),
                    None => Err(ConfigError::from_invalid_string_argument(
                        &document, node, 0,
                    )),
                },
                None => Err(ConfigError::from_missing_argument(&document, "content")),
            }?;

            let content_val = match content.get_mut(&source) {
                Some(value) => Ok(value),
                None => Err(ConfigError::from_missing_source(&document, node)),
            }?;

            if let Some(children) = node.children() {
                for child in children.nodes().iter() {
                    match child.name().value() {
                        "destination" => {
                            let destination =
                                kdl_helpers::first_string_arg(children, "destination", || {
                                    Ok("default".to_owned())
                                })
                                .unwrap();
                            content_val.destination = PathBuf::from(destination);
                        },
                        "depends_on" => {
                            for entry in child.entries() {
                                if entry.name().is_some() {
                                    continue;
                                }

                                content_val.dependencies.push(entry.value().as_string().unwrap().to_owned());
                            }
                        },
                        _ => {}
                    };
                }
            }
        }

        let variables = kdl_helpers::variables_from_kdl_document(&document)?;

        Ok(Self {
            root,
            content,
            tasks,
            variables,
            is_default,
        })
    }

    pub fn calculate(&self) -> Vec<Content> {
        let mut result: Vec<Content> = Vec::new();

        let mut collator = Collator::default();
        let mut path_keys: Vec<PathBuf> = self.content.keys().map(PathBuf::from).collect();

        path_keys.sort_by(|a, b| {
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

        let keys: Vec<String> = path_keys.clone()
            .iter()
            .map(|k| k.to_str().unwrap().to_owned())
            .collect();

        let mut dependents: HashMap<String, Vec<String>> = keys.clone()
            .iter()
            .map(|k| (k.to_owned(), vec![]))
            .collect();

        let mut dependencies = dependents.clone();

        for key in keys.clone() {
            let content = self.content.get(&key).unwrap();

            dependencies.get_mut(&key).unwrap().extend(content.dependencies.clone());
            for dep in &content.dependencies {
                dependents.get_mut(dep).unwrap().push(key.clone());
            }
        }

        let mut remaining = keys.clone();
        while !remaining.is_empty() {
            let mut count = 0;
            for key in remaining.clone() {
                if !dependencies.contains_key(&key) {
                    continue;
                }

                let content = self.content.get(&key).unwrap();
                let deps = dependencies.get_mut(&key).unwrap();

                if deps.is_empty() {
                    count += 1;
                    // push to result
                    result.push(content.clone());

                    // remove from remaining
                    remaining.retain(|name| *name != key);

                    // loop dependents to remove ourselves from their dependencies
                    for dependent in dependents.get(&key).unwrap() {
                        dependencies.get_mut(dependent).unwrap().retain(|name| *name != *key);
                    }
                }
            }

            if count == 0 {
                panic!("dependency loop detected");
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod skeleton_config {
        use super::*;
        use std::{fs, io::Write};

        use tempfile::{NamedTempFile, TempDir};
        use tera::{Number, Value};

        use crate::config::task::TaskStep;
        use crate::error::ConfigErrorKind;

        #[test]
        fn wraps_io_errors() {
            let skeleton_config = SkeletonConfig::read_from(&PathBuf::from("/etc"));
            assert!(skeleton_config.is_err());

            let is_io_error = match skeleton_config.unwrap_err() {
                SkelError::IoError(_) => true,
                _ => false,
            };
            assert!(is_io_error);
        }

        #[test]
        fn wraps_kdl_errors() {
            let mut config_file = NamedTempFile::new().unwrap();
            write!(config_file, "1.").unwrap();

            let skeleton_config = SkeletonConfig::read_from(&config_file.path().to_path_buf());
            assert!(skeleton_config.is_err());

            let is_kdl_error = match skeleton_config.unwrap_err() {
                SkelError::KdlError(_) => true,
                _ => false,
            };
            assert!(is_kdl_error);
        }

        #[test]
        fn reads_config() {
            let mut file = NamedTempFile::new().unwrap();
            write!(
                file,
                r#"variables {{
                foo "bar"
                bar 1.2
                baz 3
                oops null
                error false
            }}
            task "test" {{
              env foo="bar"
              exec "echo" "hello world"
              task "subtask" "args"
            }}"#).unwrap();

            let result = SkeletonConfig::read_from(&file.path().to_path_buf());
            assert!(result.is_ok());

            let config = result.unwrap();
            assert_eq!(config.is_default, false);

            let mut expected_context = Context::new();
            expected_context.insert("foo".to_owned(), &Value::String("bar".into()));
            expected_context.insert(
                "bar".to_owned(),
                &Value::Number(Number::from_f64(1.2).unwrap()),
            );
            expected_context.insert("baz".to_owned(), &Value::Number(3.into()));
            expected_context.insert("oops".to_owned(), &Value::Null);
            expected_context.insert("error".to_owned(), &Value::Bool(false));
            assert_eq!(config.variables, expected_context);

            assert_eq!(config.tasks.len(), 1);
            assert!(config.tasks.contains_key("test"));
            let mut env_map: HashMap<String, String> = HashMap::new();
            env_map.insert("foo".to_owned(), "bar".to_owned());
            let step_one = TaskStep::Env(env_map);
            let step_two = TaskStep::Exec("echo".to_owned(), vec!["hello world".to_owned()]);
            let step_three = TaskStep::Task("subtask".to_owned(), vec!["args".to_owned()]);
            assert_eq!(config.tasks.get("test").unwrap(), &Task {
                name: "test".to_owned(),
                steps: vec![step_one, step_two, step_three],
            });
        }

        #[test]
        fn errors_when_variable_is_missing_value() {
            let mut file = NamedTempFile::new().unwrap();
            write!(
                file,
                r#"variables {{
                foo
            }}"#
            )
            .unwrap();

            let result = SkeletonConfig::read_from(&file.path().to_path_buf());
            assert!(result.is_err());

            let is_missing_arg_error = match result.unwrap_err() {
                SkelError::ConfigError(err) => err.kind == ConfigErrorKind::MissingArgument,
                _ => false,
            };
            assert!(is_missing_arg_error);
        }

        #[test]
        fn errors_when_content_is_missing_source() {
            let mut file = NamedTempFile::new().unwrap();
            write!(
                file,
                r#"
                content
            "#
            )
            .unwrap();

            let result = SkeletonConfig::read_from(&file.path().to_path_buf());
            assert!(result.is_err());

            let is_missing_arg_error = match result.unwrap_err() {
                SkelError::ConfigError(err) => err.kind == ConfigErrorKind::MissingArgument,
                _ => false,
            };
            assert!(is_missing_arg_error);
        }

        #[test]
        fn errors_when_content_has_invalid_value() {
            let mut file = NamedTempFile::new().unwrap();
            write!(
                file,
                r#"
                content false
            "#
            )
            .unwrap();

            let result = SkeletonConfig::read_from(&file.path().to_path_buf());
            assert!(result.is_err());

            let is_invalid_string_error = match result.unwrap_err() {
                SkelError::ConfigError(err) => err.kind == ConfigErrorKind::InvalidString,
                _ => false,
            };
            assert!(is_invalid_string_error);
        }

        #[test]
        fn errors_when_source_does_not_exist() {
            let mut file = NamedTempFile::new().unwrap();
            write!(
                file,
                r#"
                content "missing/file"
            "#
            )
            .unwrap();

            let result = SkeletonConfig::read_from(&file.path().to_path_buf());
            assert!(result.is_err());

            let is_missing_source_error = match result.unwrap_err() {
                SkelError::ConfigError(err) => err.kind == ConfigErrorKind::MissingSource,
                _ => false,
            };
            assert!(is_missing_source_error);
        }

        #[test]
        fn reads_implicit_content() {
            let dir = TempDir::new().unwrap();
            fs::create_dir(dir.path().join("content")).unwrap();
            fs::write(dir.path().join("content/one"), "should be found").unwrap();

            let result = SkeletonConfig::read_from(&dir.path().join("skeleton.kdl"));
            assert!(result.is_ok());

            let skeleton = result.unwrap();
            assert!(skeleton.is_default);
            assert_eq!(skeleton.root, dir.path().join("content"));

            let mut content_map: HashMap<String, Content> = HashMap::new();
            content_map.insert(
                "one".to_owned(),
                Content::from_source(&PathBuf::from("one"), None),
            );
            assert_eq!(skeleton.content, content_map);

            assert_eq!(skeleton.variables, Context::new());
        }

        #[test]
        fn allows_overriding_destination() {
            let dir = TempDir::new().unwrap();
            fs::create_dir(dir.path().join("content")).unwrap();
            fs::write(dir.path().join("content/one"), "should be found").unwrap();

            fs::write(
                dir.path().join("skeleton.kdl"),
                r#"
                content "one" {
                    destination "two"
                }
            "#,
            )
            .unwrap();

            let result = SkeletonConfig::read_from(&dir.path().join("skeleton.kdl"));
            assert!(result.is_ok());

            let skeleton = result.unwrap();
            assert_eq!(skeleton.is_default, false);
            assert_eq!(skeleton.root, dir.path().join("content"));

            let mut content_map: HashMap<String, Content> = HashMap::new();
            let mut content = Content::from_source(&PathBuf::from("one"), None);
            content.destination = PathBuf::from("two");
            content_map.insert("one".to_owned(), content);
            assert_eq!(skeleton.content, content_map);

            assert_eq!(skeleton.variables, Context::new());
        }

        #[test]
        fn allows_declaring_dependencies() {
            let dir = TempDir::new().unwrap();
            fs::create_dir(dir.path().join("content")).unwrap();
            fs::write(dir.path().join("content/one"), "should be third").unwrap();
            fs::write(dir.path().join("content/two"), "should be second").unwrap();
            fs::write(dir.path().join("content/three"), "should be first").unwrap();

            fs::write(dir.path().join("skeleton.kdl"), r#"
            content "one" {
                depends_on "two" "three"
            }
            content "two" {
                depends_on "three"
            }
            "#).unwrap();

            let result = SkeletonConfig::read_from(&dir.path().join("skeleton.kdl"));
            assert!(result.is_ok());

            let skeleton = result.unwrap();
            assert_eq!(skeleton.is_default, false);
            assert_eq!(skeleton.root, dir.path().join("content"));

            let mut content_map: HashMap<String, Content> = HashMap::new();

            let mut content_one = Content::from_source(&PathBuf::from("one"), None);
            content_one.dependencies.push("two".to_owned());
            content_one.dependencies.push("three".to_owned());
            content_map.insert("one".to_owned(), content_one);

            let mut content_two = Content::from_source(&PathBuf::from("two"), None);
            content_two.dependencies.push("three".to_owned());
            content_map.insert("two".to_owned(), content_two);

            let content_three = Content::from_source(&PathBuf::from("three"), None);
            content_map.insert("three".to_owned(), content_three);

            assert_eq!(skeleton.content, content_map);

            let steps = skeleton.calculate();
            assert_eq!(steps.len(), 3);
            assert_eq!(steps[0].source, PathBuf::from("three"));
            assert_eq!(steps[1].source, PathBuf::from("two"));
            assert_eq!(steps[2].source, PathBuf::from("one"));
        }

        #[test]
        fn ignores_non_destination_children() {
            let dir = TempDir::new().unwrap();
            fs::create_dir(dir.path().join("content")).unwrap();
            fs::write(dir.path().join("content/one"), "should be found").unwrap();

            fs::write(
                dir.path().join("skeleton.kdl"),
                r#"
                content "one" {
                    not_destination "two"
                }
            "#,
            )
            .unwrap();

            let result = SkeletonConfig::read_from(&dir.path().join("skeleton.kdl"));
            assert!(result.is_ok());

            let skeleton = result.unwrap();
            assert_eq!(skeleton.is_default, false);
            assert_eq!(skeleton.root, dir.path().join("content"));

            let mut content_map: HashMap<String, Content> = HashMap::new();
            let content = Content::from_source(&PathBuf::from("one"), None);
            content_map.insert("one".to_owned(), content);
            assert_eq!(skeleton.content, content_map);

            assert_eq!(skeleton.variables, Context::new());
        }
    }
}
