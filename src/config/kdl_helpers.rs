use kdl::{KdlDocument, KdlEntry, KdlValue};
use tera::{Context, Number, Value};

use crate::error::{ConfigError, SkelError};

pub fn first_string_arg<F>(
    document: &KdlDocument,
    name: &str,
    default: F,
) -> Result<String, SkelError>
where
    F: FnOnce() -> Result<String, SkelError>,
{
    let node = document.get(name);
    if node.is_some() {
        match node.unwrap().get(0) {
            Some(entry) => match entry.value().as_string() {
                Some(value) => Ok(value.to_owned()),
                None => Err(
                    ConfigError::from_invalid_string_argument(document, node.unwrap(), 0).into(),
                ),
            },
            None => Err(ConfigError::from_missing_argument(document, name).into()),
        }
    } else {
        default()
    }
}

pub fn kdl_entry_to_tera_value(entry: &KdlEntry) -> Value {
    match entry.value().to_owned() {
        KdlValue::RawString(s) | KdlValue::String(s) => Value::String(s.to_owned()),
        KdlValue::Base2(i) | KdlValue::Base8(i) | KdlValue::Base10(i) | KdlValue::Base16(i) => Value::Number(i.into()),
        KdlValue::Base10Float(f) => Value::Number(Number::from_f64(f).unwrap()),
        KdlValue::Bool(b) => Value::Bool(b),
        KdlValue::Null => Value::Null,
    }
}

pub fn kdl_entry_to_string(entry: &KdlEntry) -> String {
    match entry.value() {
        KdlValue::RawString(s) | KdlValue::String(s) => s.to_owned(),
        KdlValue::Base2(i) | KdlValue::Base8(i) | KdlValue::Base10(i) | KdlValue::Base16(i) => i.to_string(),
        KdlValue::Base10Float(f) => f.to_string(),
        KdlValue::Bool(b) => b.to_string(),
        KdlValue::Null => "null".to_owned(),
    }
}

pub fn variables_from_kdl_document(doc: &KdlDocument) -> Result<Context, SkelError> {
    let mut variables = Context::new();

    let node_opt = doc.get("variables");
    if node_opt.is_none() {
        return Ok(variables);
    }

    let node = node_opt.unwrap();
    let children_opt = node.children();
    if children_opt.is_none() {
        return Ok(variables);
    }

    let children = children_opt.unwrap();
    for node in children.nodes() {
        let name = node.name().value().to_owned();
        let entry_opt = node.get(0);
        if entry_opt.is_none() {
            return Err(ConfigError::from_missing_argument(children, &name).into());
        }

        let entry = entry_opt.unwrap();
        variables.insert(name, &kdl_entry_to_tera_value(entry));
    }

    Ok(variables)
}

#[cfg(test)]
mod tests {
    use super::*;
    mod first_string_arg_helper {
        use super::*;
        use crate::error::ConfigErrorKind;

        #[test]
        fn returns_the_first_arg() {
            let doc: KdlDocument = "root \"/\"".parse().unwrap();
            let result = first_string_arg(&doc, "root", || Ok("default".to_owned()));
            assert!(result.is_ok());

            let value = result.unwrap();
            assert_eq!(value, "/".to_owned());
        }

        #[test]
        fn returns_default_for_missing_node() {
            let doc: KdlDocument = "".parse().unwrap();
            let result = first_string_arg(&doc, "root", || Ok("default".to_owned()));
            assert!(result.is_ok());

            let value = result.unwrap();
            assert_eq!(value, "default".to_owned());
        }

        #[test]
        fn errors_for_missing_argument() {
            let doc: KdlDocument = "root".parse().unwrap();
            let result = first_string_arg(&doc, "root", || Ok("default".to_owned()));
            assert!(result.is_err());

            let is_missing_arg_error = match result.unwrap_err() {
                SkelError::ConfigError(err) => err.kind == ConfigErrorKind::MissingArgument,
                _ => false,
            };
            assert!(is_missing_arg_error);
        }

        #[test]
        fn errors_for_non_string_argument() {
            let doc: KdlDocument = "root 1.2".parse().unwrap();
            let result = first_string_arg(&doc, "root", || Ok("default".to_owned()));
            assert!(result.is_err());

            let is_invalid_string_error = match result.unwrap_err() {
                SkelError::ConfigError(err) => err.kind == ConfigErrorKind::InvalidString,
                _ => false,
            };
            assert!(is_invalid_string_error);
        }
    }
}
