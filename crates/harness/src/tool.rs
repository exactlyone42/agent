use crate::prelude::*;
use schemars::{JsonSchema, generate::SchemaSettings};
use serde_json::Map;
use std::pin::Pin;

#[derive(Clone, Debug, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl ToolSpec {
    pub fn typed<T: JsonSchema>(name: &str, description: &str) -> Self {
        let settings = SchemaSettings::draft2020_12().with(|s| s.inline_subschemas = true);
        let mut parameters = settings
            .into_generator()
            .into_root_schema_for::<T>()
            .to_value();
        if let Some(object) = parameters.as_object_mut() {
            object.remove("$schema");
            object.remove("title");
            object
                .entry("properties")
                .or_insert_with(|| Value::Object(Map::new()));
        }
        Self {
            name: name.to_owned(),
            description: description.to_owned(),
            parameters,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ToolError(pub String);

impl ToolError {
    pub fn refused(why: &str) -> Self {
        Self(why.to_owned())
    }

    pub fn failed(error: impl std::error::Error) -> Self {
        Self(error.to_string())
    }
}

pub type ToolFuture<'a> = Pin<Box<dyn Future<Output = Result<String, ToolError>> + Send + 'a>>;

pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    fn call(&self, args: Value) -> ToolFuture<'_>;
}
