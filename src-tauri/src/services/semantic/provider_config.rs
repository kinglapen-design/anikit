use keyring::{Entry, Error};

use super::provider_types::{ProviderConfigView, ProviderSettings};
use crate::core::config::{save_semantic_provider_settings, semantic_provider_settings};

const KEYRING_SERVICE: &str = "AniKit";
const KEYRING_ACCOUNT: &str = "semantic-provider";

fn key_entry() -> Result<Entry, String> {
  Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).map_err(|_| "无法访问 Windows 钥匙串".into())
}

fn read_key() -> Result<Option<String>, String> {
  match key_entry()?.get_password() {
    Ok(key) => Ok(Some(key)),
    Err(Error::NoEntry) => Ok(None),
    Err(_) => Err("读取 Windows 钥匙串中的 provider 密钥失败".into()),
  }
}

pub fn get_view() -> Result<ProviderConfigView, String> {
  Ok(ProviderConfigView { settings: settings()?, has_key: read_key()?.is_some() })
}

pub fn save(mut settings: ProviderSettings) -> Result<(), String> {
  settings.endpoint = settings.endpoint.trim().to_string();
  settings.model = settings.model.trim().to_string();
  settings.validate()?;
  save_semantic_provider_settings(settings).map_err(|e| format!("保存外部 provider 配置失败：{e}"))
}

pub fn set_key(key: String) -> Result<(), String> {
  if key.trim().is_empty() {
    return Err("API Key 不能为空".into());
  }
  key_entry()?.set_password(&key).map_err(|_| "写入 Windows 钥匙串失败".into())
}

pub fn clear_key() -> Result<(), String> {
  match key_entry()?.delete_credential() {
    Ok(()) | Err(Error::NoEntry) => Ok(()),
    Err(_) => Err("清除 Windows 钥匙串中的 provider 密钥失败".into()),
  }
}

pub(crate) fn settings() -> Result<ProviderSettings, String> {
  let settings: ProviderSettings = semantic_provider_settings().map_err(|e| format!("读取外部 provider 配置失败：{e}"))?;
  settings.validate()?;
  Ok(settings)
}

pub(crate) fn api_key() -> Result<Option<String>, String> {
  read_key()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn persisted_settings_have_no_secret_field() {
    let settings = ProviderSettings { endpoint: "https://example.com/v1".into(), model: "secret-looking-model".into(), ..Default::default() };
    let value = serde_json::to_value(settings).unwrap();
    let keys: std::collections::BTreeSet<_> = value.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["endpoint", "kind", "model", "timeoutSecs"].into());
    let view = ProviderConfigView { settings: ProviderSettings::default(), has_key: true };
    let view = serde_json::to_value(view).unwrap();
    assert_eq!(view["hasKey"], true);
    assert!(view.get("apiKey").is_none());
    assert!(view.get("key").is_none());
  }

  #[test]
  fn keyring_entry_identity_does_not_include_endpoint() {
    assert_eq!(KEYRING_SERVICE, "AniKit");
    assert_eq!(KEYRING_ACCOUNT, "semantic-provider");
  }

  #[test]
  #[ignore = "写入并立即清除测试专用 Windows 凭据"]
  fn windows_keyring_dummy_roundtrip() {
    let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let account = format!("semantic-provider-test-{}-{nonce}", std::process::id());
    let entry = Entry::new(KEYRING_SERVICE, &account).unwrap();
    entry.set_password("dummy-not-a-secret").unwrap();
    let read = entry.get_password();
    let deleted = entry.delete_credential();
    assert_eq!(read.as_deref().unwrap(), "dummy-not-a-secret");
    deleted.unwrap();
    assert!(matches!(entry.get_password(), Err(Error::NoEntry)));
  }
}
