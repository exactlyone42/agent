use agent_harness::ModelError;

pub(crate) fn unsupported() -> ModelError {
    ModelError("模型回應含有無法保存或續接的內容".into())
}
