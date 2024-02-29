use kdl::{KdlDocument, KdlEntry, KdlNode};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum SkelError {
    #[error(transparent)]
    #[diagnostic(code(skel::io_error))]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(transparent)]
    KdlError(#[from] kdl::KdlError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    ConfigError(#[from] ConfigError),

    #[error("{0}")]
    #[diagnostic(code(skel::other_error))]
    Other(String),
}

#[derive(Clone, Debug, Diagnostic, Error, Eq, PartialEq)]
#[error("{kind}")]
pub struct ConfigError {
    #[source_code]
    pub config: String,
    #[label("{}", label.unwrap_or("here"))]
    pub span: SourceSpan,
    pub label: Option<&'static str>,
    #[help]
    pub help: Option<&'static str>,
    pub kind: ConfigErrorKind,
}

#[derive(Clone, Debug, Diagnostic, Error, Eq, PartialEq)]
pub enum ConfigErrorKind {
    #[error("missing required argument")]
    #[diagnostic(code(skel::config::missing_required_arg))]
    MissingArgument,

    #[error("invalid string value")]
    #[diagnostic(code(skel::config::invalid_string))]
    InvalidString,

    #[error("missing source file")]
    #[diagnostic(code(skel::config::missing_source))]
    MissingSource,

    #[error("invalid float")]
    #[diagnostic(code(skel::config::invalid_float))]
    InvalidFloat,
}

impl ConfigError {
    pub fn from_missing_argument(doc: &KdlDocument, node_name: &str) -> Self {
        let mut my_doc = doc.clone();
        let node = my_doc.get_mut(node_name).unwrap();
        let entries = node.entries_mut();
        entries.insert(0, KdlEntry::new("ARG"));

        let proposed_doc = my_doc.to_string();
        let reparsed: KdlDocument = proposed_doc.parse().unwrap();
        let reparsed_node = reparsed.get(node_name).unwrap();
        let inserted_entry = reparsed_node.get(0).unwrap();

        Self {
            config: proposed_doc,
            span: inserted_entry.span().to_owned(),
            help: Some("this node requires an argument"),
            label: Some("insert an argument here"),
            kind: ConfigErrorKind::MissingArgument,
        }
    }

    pub fn from_invalid_string_argument(doc: &KdlDocument, node: &KdlNode, index: usize) -> Self {
        let span = node.get(index).unwrap().span().to_owned();

        Self {
            config: doc.to_string(),
            span,
            help: Some("the indicated argument must be a string"),
            label: None,
            kind: ConfigErrorKind::InvalidString,
        }
    }

    pub fn from_missing_source(doc: &KdlDocument, node: &KdlNode) -> Self {
        Self {
            config: doc.to_string(),
            span: node.get(0).unwrap().span().to_owned(),
            help: Some("the file indicated does not exist"),
            label: None,
            kind: ConfigErrorKind::MissingSource,
        }
    }
}
