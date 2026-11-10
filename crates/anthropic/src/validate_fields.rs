use agent_harness::ModelError;
use serde_json::Value;

pub(crate) fn validate_fields(value: &Value, allowed: &[&str]) -> Result<(), ModelError> {
    let bad = || ModelError("模型回應含有無法保存或續接的內容".into());
    let fields = value.as_object().ok_or_else(bad)?;
    if fields
        .iter()
        .any(|(key, value)| !allowed.contains(&key.as_str()) && !value.is_null())
    {
        return Err(bad());
    }
    Ok(())
}
