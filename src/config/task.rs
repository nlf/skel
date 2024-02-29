use std::collections::HashMap;

use kdl::KdlDocument;

use crate::config::kdl_helpers;
use crate::error::{ConfigError, SkelError};

#[derive(Debug, Eq, PartialEq)]
pub struct Task {
    pub name: String,
    pub steps: Vec<TaskStep>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TaskStep {
    Env(HashMap<String, String>),
    Exec(String, Vec<String>),
    Task(String, Vec<String>),
}

impl Task {
    pub fn from_kdl_document(doc: &KdlDocument, name: String) -> Result<Self, SkelError> {
        let mut steps: Vec<TaskStep> = Vec::new();

        for node in doc.nodes().iter() {
            match node.name().value() {
                "env" => {
                    let mut vars: HashMap<String, String> = HashMap::new();

                    for entry in node.entries().iter().filter(|e| e.name().is_some()) {
                        if entry.name().is_none() {
                            continue;
                        }

                        let name = String::from(entry.name().unwrap().value());
                        let value = kdl_helpers::kdl_entry_to_string(entry);
                        vars.insert(name, value);
                    }

                    steps.push(TaskStep::Env(vars));
                }
                "exec" => {
                    let mut command = "".to_owned();
                    let mut args: Vec<String> = Vec::new();

                    for entry in node.entries() {
                        if entry.name().is_some() {
                            continue;
                        }

                        if command.is_empty() {
                            command = match entry.value().as_string() {
                                Some(value) => Ok(value.to_owned()),
                                None => Err(ConfigError::from_missing_argument(doc, "exec")),
                            }?;
                            continue;
                        }

                        let arg = kdl_helpers::kdl_entry_to_string(entry);
                        args.push(arg);
                    }

                    steps.push(TaskStep::Exec(command, args));
                }
                "task" => {
                    let mut task = "".to_owned();
                    let mut args: Vec<String> = Vec::new();

                    for entry in node.entries() {
                        if entry.name().is_some() {
                            continue;
                        }

                        if task.is_empty() {
                            task = match entry.value().as_string() {
                                Some(value) => Ok(value.to_owned()),
                                None => Err(ConfigError::from_missing_argument(doc, "task")),
                            }?;
                            continue;
                        }

                        let arg = kdl_helpers::kdl_entry_to_string(entry);
                        args.push(arg);
                    }

                    steps.push(TaskStep::Task(task, args));
                }
                _ => {}
            };
        }

        Ok(Task { name, steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    mod from_kdl_document {
        use super::*;
        
        #[test]
        fn can_create_a_task() {
            let doc: KdlDocument = r#"
                env bool=true int=1 float=2.3 str="string" nil=null
                exec "command" "arg1" "arg2"
                env bool=false int=2 float=3.4 str="different_string"
                task "task" "arg1" "arg2"
            "#.parse().unwrap();

            let task = Task::from_kdl_document(&doc, "test".to_owned()).unwrap();
            assert_eq!(task.name, "test".to_owned());
            assert_eq!(task.steps.len(), 4);

            let mut first_env: HashMap<String, String> = HashMap::new();
            first_env.insert("bool".to_owned(), "true".to_owned());
            first_env.insert("int".to_owned(), "1".to_owned());
            first_env.insert("float".to_owned(), "2.3".to_owned());
            first_env.insert("str".to_owned(), "string".to_owned());
            first_env.insert("nil".to_owned(), "null".to_owned());
            assert_eq!(task.steps[0], TaskStep::Env(first_env));

            assert_eq!(task.steps[1], TaskStep::Exec("command".to_owned(), vec!["arg1".to_owned(), "arg2".to_owned()]));

            let mut second_env: HashMap<String, String> = HashMap::new();
            second_env.insert("bool".to_owned(), "false".to_owned());
            second_env.insert("int".to_owned(), "2".to_owned());
            second_env.insert("float".to_owned(), "3.4".to_owned());
            second_env.insert("str".to_owned(), "different_string".to_owned());
            assert_eq!(task.steps[2], TaskStep::Env(second_env));

            assert_eq!(task.steps[3], TaskStep::Task("task".to_owned(), vec!["arg1".to_owned(), "arg2".to_owned()]));
        }
    }
}
