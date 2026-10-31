use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProviderKind {
  #[default]
  #[serde(rename = "openai-compatible")]
  OpenAiCompatible,
  #[serde(rename = "ollama")]
  Ollama,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProviderSettings {
  pub kind: ProviderKind,
  pub endpoint: String,
  pub model: String,
  pub timeout_secs: u32,
}

impl Default for ProviderSettings {
  fn default() -> Self {
    Self { kind: ProviderKind::OpenAiCompatible, endpoint: String::new(), model: String::new(), timeout_secs: 90 }
  }
}

impl ProviderSettings {
  pub(crate) fn validate(&self) -> Result<(), String> {
    self.endpoint_url().map(|_| ())
  }

  pub(crate) fn require_configured(&self) -> Result<Url, String> {
    self.endpoint_url()?.ok_or_else(|| "外部视频理解 provider 尚未配置 endpoint 和 model".into())
  }

  fn endpoint_url(&self) -> Result<Option<Url>, String> {
    if self.timeout_secs == 0 {
      return Err("timeoutSecs 必须大于 0".into());
    }
    let endpoint = self.endpoint.trim();
    let model = self.model.trim();
    if endpoint.is_empty() || model.is_empty() {
      return if endpoint.is_empty() && model.is_empty() {
        Ok(None)
      } else {
        Err("endpoint 和 model 必须同时填写；两者留空表示未配置".into())
      };
    }
    let url = Url::parse(endpoint).map_err(|_| "endpoint 不是有效的 http(s) URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
      return Err("endpoint 仅支持 http(s) URL".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
      return Err("endpoint 不允许内嵌用户名或密码".into());
    }
    if url.scheme() == "http" && !is_loopback(&url) {
      return Err("HTTP endpoint 仅允许 localhost、127.0.0.1 或 [::1]；对外服务请使用 HTTPS".into());
    }
    Ok(Some(url))
  }
}

fn is_loopback(url: &Url) -> bool {
  url.host_str().is_some_and(|host| {
    host.eq_ignore_ascii_case("localhost") || matches!(host, "127.0.0.1" | "[::1]" | "::1")
  })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigView {
  pub settings: ProviderSettings,
  pub has_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConnection {
  pub ok: bool,
  pub message: String,
  pub latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderEvent {
  pub text: String,
  pub start_ms: i64,
  pub end_ms: i64,
  pub certainty: String,
  pub evidence: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticDocumentInput {
  pub kind: String,
  pub text: String,
  pub start_ms: i64,
  pub end_ms: i64,
  pub start_frame: Option<u32>,
  pub end_frame: Option<u32>,
  pub certainty: String,
  pub provider: String,
  pub model: String,
  pub prompt_version: String,
  pub source_json: String,
  pub frame_ms: Vec<i64>,
  pub vector: Vec<f32>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn defaults_and_serde_names_match_contract() {
    let defaults = ProviderSettings::default();
    assert_eq!(defaults.timeout_secs, 90);
    assert_eq!(defaults.kind, ProviderKind::OpenAiCompatible);
    let value = serde_json::to_value(defaults).unwrap();
    assert_eq!(value, serde_json::json!({"kind":"openai-compatible","endpoint":"","model":"","timeoutSecs":90}));
    assert_eq!(serde_json::to_value(ProviderKind::Ollama).unwrap(), "ollama");
    let event = ProviderEvent { text: "起身".into(), start_ms: 10, end_ms: 20, certainty: "observed".into(), evidence: vec![0] };
    assert!(serde_json::to_value(event).unwrap().get("startMs").is_some());
  }

  #[test]
  fn endpoint_rules_reject_insecure_or_credentialed_urls() {
    let valid = |endpoint: &str| ProviderSettings { endpoint: endpoint.into(), model: "m".into(), ..Default::default() };
    for endpoint in ["https://api.openai.com/v1", "http://localhost:11434", "http://127.0.0.1:8000", "http://[::1]:8000"] {
      assert!(valid(endpoint).validate().is_ok(), "{endpoint}");
    }
    for endpoint in ["file:///tmp/model", "https://user:pass@example.com/v1", "http://example.com/v1", "http://localhost.evil.com"] {
      assert!(valid(endpoint).validate().is_err(), "{endpoint}");
    }
    assert!(ProviderSettings::default().validate().is_ok());
    assert!(valid("https://example.com").require_configured().is_ok());
    assert!(ProviderSettings { model: "m".into(), ..Default::default() }.validate().is_err());
    assert!(ProviderSettings { timeout_secs: 0, ..Default::default() }.validate().is_err());
  }
}
