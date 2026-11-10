use crate::unsupported::unsupported;
use agent_harness::ModelError;
use serde_json::Value;

pub(crate) fn validate_fields(value: &Value, allowed: &[&str]) -> Result<(), ModelError> {
    let fields = value.as_object().ok_or_else(unsupported)?;
    if fields
        .iter()
        .any(|(key, value)| !allowed.contains(&key.as_str()) && !value.is_null())
    {
        return Err(unsupported());
    }
    Ok(())
}
