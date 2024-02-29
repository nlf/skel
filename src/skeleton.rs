use std::collections::HashMap;
use std::default::Default;
use std::path::PathBuf;
use tera::Context;

use crate::config::{ProjectConfig, SkeletonConfig, Task};
use crate::content::Content;
use crate::error::SkelError;

#[derive(Debug, Default)]
pub struct Skeleton {
    pub project: PathBuf,
    pub skeleton: PathBuf,
    pub content: HashMap<String, Content>,
    pub variables: Context,
    pub tasks: HashMap<String, Task>,
}

impl Skeleton {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_config_file(config_file: PathBuf) -> Result<Self, SkelError> {
        let project_config = ProjectConfig::read_from(&config_file)?;
        let skeleton_config = SkeletonConfig::read_from(&project_config.skeleton.join("skeleton.kdl"))?;

        let mut variables = Context::new();
        variables.extend(skeleton_config.variables);
        variables.extend(project_config.variables);

        let mut tasks: HashMap<String, Task> = HashMap::new();
        for (key, value) in skeleton_config.tasks {
            tasks.insert(key, value);
        }
        for (key, value) in project_config.tasks {
            tasks.insert(key, value);
        }

        Ok(Self {
            project: project_config.root,
            skeleton: project_config.skeleton,
            content: skeleton_config.content,
            variables,
            tasks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn creates_a_default_skeleton() {
        let skeleton = Skeleton::new();
        assert_eq!(
            skeleton.project,
            PathBuf::from("")
        );
        assert_eq!(
            skeleton.skeleton,
            PathBuf::from("")
        );
        assert_eq!(skeleton.content, HashMap::new());
        assert_eq!(skeleton.variables, Context::new());
        assert_eq!(skeleton.tasks, HashMap::new());
    }

    #[test]
    fn creates_a_default_from_missing_config() {
        let config_path = TempDir::new().unwrap().path().to_owned();
        let skeleton = Skeleton::from_config_file(config_path.join("skeleton.kdl")).unwrap();
        assert_eq!(skeleton.project, config_path);
        assert_eq!(skeleton.skeleton, config_path.join(".skeleton"));
        assert_eq!(skeleton.content, HashMap::new());
        assert_eq!(skeleton.variables, Context::new());
        assert_eq!(skeleton.tasks, HashMap::new());
    }
}
