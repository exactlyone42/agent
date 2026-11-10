use agent_harness::{Message, Model, ModelError, ModelReply, ToolSpec};
use serde_json::Value;
use std::time::Duration;
use url::Url;

pub trait Wire: Send + Sync {
    fn path(&self) -> &'static str;
    fn headers(&self, key: &str) -> Vec<(&'static str, String)>;
    fn request(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[ToolSpec],
    ) -> Result<Value, ModelError>;
    fn reply(&self, body: &Value) -> Result<ModelReply, ModelError>;
}

pub struct HttpModel<W: Wire> {
    http: reqwest::Client,
    url: Url,
    model: String,
    key: String,
    wire: W,
}

impl<W: Wire> HttpModel<W> {
    pub fn new(
        wire: W,
        base: &Url,
        model: impl Into<String>,
        key: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, ModelError> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(timeout)
            .build()
            .map_err(|_| ModelError("無法建立模型連線".into()))?;
        let mut url = base.clone();
        let path = format!("{}/{}", base.path().trim_end_matches('/'), wire.path());
        url.set_path(&path);
        Ok(Self {
            http,
            url,
            model: model.into(),
            key: key.into(),
            wire,
        })
    }
}

impl<W: Wire> Model for HttpModel<W> {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
    ) -> Result<ModelReply, ModelError> {
        let body = self.wire.request(&self.model, messages, tools)?;
        let mut request = self.http.post(self.url.clone()).json(&body);
        for (name, value) in self.wire.headers(&self.key) {
            request = request.header(name, value);
        }
        let response = request
            .send()
            .await
            .map_err(|_| ModelError("無法連線模型服務".into()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(ModelError(format!("模型服務回應 {status}")));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|_| ModelError("模型回應不是有效 JSON".into()))?;
        self.wire.reply(&body)
    }
}
