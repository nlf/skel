use std::collections::HashMap;
use std::path::PathBuf;

use kdl::KdlDocument;
use tera::Context;

use crate::config::fs_helpers::read_to_string_with_default;
use crate::config::kdl_helpers::{first_string_arg, variables_from_kdl_document};
use crate::config::task::Task;
use crate::error::{ConfigError, SkelError};

#[derive(Debug, Default)]
pub struct ProjectConfig {
    pub root: PathBuf,
    pub skeleton: PathBuf,
    pub variables: Context,
    pub tasks: HashMap<String, Task>,
    pub is_default: bool,
}

impl ProjectConfig {
    pub fn read_from(path: &PathBuf) -> Result<Self, SkelError> {
        let (config_content, is_default) = read_to_string_with_default(path)?;
        let document: KdlDocument = config_content.parse()?;

        let default_root = || Ok(path.parent().unwrap().to_string_lossy().into_owned());
        let root_str = first_string_arg(&document, "root", default_root)?;
        let root = PathBuf::from(root_str);

        let default_skeleton = || Ok(root.join(".skeleton").to_string_lossy().into_owned());
        let skeleton_str = first_string_arg(&document, "skeleton", default_skeleton)?;
        let skeleton = PathBuf::from(skeleton_str);

        let variables = variables_from_kdl_document(&document)?;

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

        Ok(Self {
            root,
            skeleton,
            variables,
            tasks,
            is_default,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod project_config {
        use super::*;
        use std::io::Write;

        use tempfile::NamedTempFile;
        use tera::{Number, Value};

        use crate::config::task::TaskStep;
        use crate::error::ConfigErrorKind;

        #[test]
        fn wraps_io_errors() {
            let project_config = ProjectConfig::read_from(&PathBuf::from("/"));
            assert!(project_config.is_err());

            let is_io_error = match project_config.unwrap_err() {
                SkelError::IoError(_) => true,
                _ => false,
            };
            assert!(is_io_error);
        }

        #[test]
        fn wraps_kdl_errors() {
            let mut config_file = NamedTempFile::new().unwrap();
            write!(config_file, "1.").unwrap();

            let project_config = ProjectConfig::read_from(&config_file.path().to_path_buf());
            assert!(project_config.is_err());

            let is_kdl_error = match project_config.unwrap_err() {
                SkelError::KdlError(_) => true,
                _ => false,
            };
            assert!(is_kdl_error);
        }

        #[test]
        fn defaults() {
            let result = ProjectConfig::read_from(&PathBuf::from("/A/PATH/THAT/DOES/NOT/EXIST"));
            assert!(result.is_ok());

            let config = result.unwrap();
            assert_eq!(config.is_default, true);
            // parent of the path passed as the config file
            assert_eq!(config.root, PathBuf::from("/A/PATH/THAT/DOES/NOT"));
            // parent of the config path + .skeleton
            assert_eq!(
                config.skeleton,
                PathBuf::from("/A/PATH/THAT/DOES/NOT/.skeleton")
            );
            assert_eq!(config.variables, Context::new());
        }

        #[test]
        fn reads_config_file() {
            let mut file = NamedTempFile::new().unwrap();
            write!(
                file,
                r#"root "/"
            skeleton "/etc/skeleton"
            variables {{
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

            let result = ProjectConfig::read_from(&file.path().to_path_buf());
            assert!(result.is_ok());

            let config = result.unwrap();
            assert_eq!(config.is_default, false);
            assert_eq!(config.root, PathBuf::from("/"));
            assert_eq!(config.skeleton, PathBuf::from("/etc/skeleton"));

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

            let result = ProjectConfig::read_from(&file.path().to_path_buf());
            assert!(result.is_err());

            let is_missing_arg_error = match result.unwrap_err() {
                SkelError::ConfigError(err) => err.kind == ConfigErrorKind::MissingArgument,
                _ => false,
            };
            assert!(is_missing_arg_error);
        }
    }
}
