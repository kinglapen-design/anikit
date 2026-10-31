use std::time::{Duration, Instant};

use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::provider_config;
use super::provider_types::{ProviderConnection, ProviderKind, ProviderSettings};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceImage {
  pub timestamp_ms: i64,
  pub source_frame: Option<u32>,
  pub png_base64: String,
}

#[derive(Clone, Copy)]
enum Operation {
  Models,
  Chat,
}

fn operation_url(settings: &ProviderSettings, operation: Operation) -> Result<String, String> {
  let mut url = settings.require_configured()?;
  let path = url.path().trim_end_matches('/').to_string();
  let path = match (settings.kind, operation) {
    (ProviderKind::OpenAiCompatible, Operation::Models) if path.ends_with("/models") => path,
    (ProviderKind::OpenAiCompatible, Operation::Models) => {
      let base = path.strip_suffix("/chat/completions").unwrap_or(&path);
      append_path(base, "models")
    }
    (ProviderKind::OpenAiCompatible, Operation::Chat) if path.ends_with("/chat/completions") => path,
    (ProviderKind::OpenAiCompatible, Operation::Chat) => append_path(&path, "chat/completions"),
    (ProviderKind::Ollama, Operation::Models) if path.ends_with("/api/tags") => path,
    (ProviderKind::Ollama, Operation::Models) => {
      if let Some(base) = path.strip_suffix("/api/chat") {
        append_path(&format!("{base}/api"), "tags")
      } else if path.ends_with("/api") {
        append_path(&path, "tags")
      } else {
        append_path(&path, "api/tags")
      }
    }
    (ProviderKind::Ollama, Operation::Chat) if path.ends_with("/api/chat") => path,
    (ProviderKind::Ollama, Operation::Chat) => {
      if let Some(base) = path.strip_suffix("/api/tags") {
        append_path(&format!("{base}/api"), "chat")
      } else if path.ends_with("/api") {
        append_path(&path, "chat")
      } else {
        append_path(&path, "api/chat")
      }
    }
  };
  url.set_path(&path);
  Ok(url.to_string())
}

fn append_path(base: &str, suffix: &str) -> String {
  format!("{}/{}", base.trim_end_matches('/'), suffix)
}

fn client(settings: &ProviderSettings) -> Result<Client, String> {
  Client::builder()
    .timeout(Duration::from_secs(u64::from(settings.timeout_secs)))
    .redirect(reqwest::redirect::Policy::none())
    .build()
    .map_err(|_| "创建外部 provider HTTP 客户端失败".into())
}

fn authorized(request: RequestBuilder, api_key: Option<&str>) -> RequestBuilder {
  match api_key {
    Some(key) => request.bearer_auth(key),
    None => request,
  }
}

fn send(request: RequestBuilder) -> Result<Response, String> {
  request.send().map_err(|error| {
    if error.is_timeout() {
      "请求外部 provider 超时，请检查服务状态与 timeoutSecs".into()
    } else if error.is_connect() {
      "无法连接外部 provider，请确认服务已启动且地址可访问".into()
    } else {
      "外部 provider HTTP 请求失败，请检查 endpoint 与网络连接".into()
    }
  })
}

pub fn test_connection(settings: &ProviderSettings) -> Result<ProviderConnection, String> {
  settings.require_configured()?;
  let key = provider_config::api_key()?;
  test_connection_with_key(settings, key.as_deref())
}

fn test_connection_with_key(settings: &ProviderSettings, api_key: Option<&str>) -> Result<ProviderConnection, String> {
  let url = operation_url(settings, Operation::Models)?;
  let started = Instant::now();
  let response = match send(authorized(client(settings)?.get(url), api_key)) {
    Ok(response) => response,
    Err(message) => return Ok(connection(false, message, started)),
  };
  let status = response.status();
  if !status.is_success() {
    let path = match settings.kind {
      ProviderKind::OpenAiCompatible => "/models",
      ProviderKind::Ollama => "/api/tags",
    };
    return Ok(connection(false, status_message(status, "连接测试", path), started));
  }
  let payload = match response.json::<Value>() {
    Ok(payload) => payload,
    Err(_) => return Ok(connection(false, "服务已响应，但模型列表格式不兼容，不能判定连接成功".into(), started)),
  };
  match model_listed(settings, &payload) {
    Ok(true) => Ok(connection(true, format!("连接成功，已找到模型「{}」", settings.model.trim()), started)),
    Ok(false) => Ok(connection(false, format!("服务已连接，但模型列表中未找到「{}」，请检查 model", settings.model.trim()), started)),
    Err(message) => Ok(connection(false, message, started)),
  }
}

fn connection(ok: bool, message: String, started: Instant) -> ProviderConnection {
  let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
  ProviderConnection { ok, message, latency_ms }
}

fn model_listed(settings: &ProviderSettings, payload: &Value) -> Result<bool, String> {
  let entries = match settings.kind {
    ProviderKind::OpenAiCompatible => payload.get("data"),
    ProviderKind::Ollama => payload.get("models"),
  }
  .and_then(Value::as_array)
  .ok_or_else(|| "服务已响应，但模型列表格式不兼容，不能判定连接成功".to_string())?;
  let model = settings.model.trim();
  Ok(entries.iter().any(|entry| match settings.kind {
    ProviderKind::OpenAiCompatible => entry.get("id").and_then(Value::as_str) == Some(model),
    ProviderKind::Ollama => ["name", "model"].iter().any(|field| {
      entry.get(*field).and_then(Value::as_str).is_some_and(|name| name == model || name.strip_suffix(":latest") == Some(model))
    }),
  }))
}

fn status_message(status: StatusCode, action: &str, path: &str) -> String {
  if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
    "鉴权失败，请检查 API Key 或服务访问权限".into()
  } else if matches!(status, StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_IMPLEMENTED) {
    format!("服务不支持所需接口 {path}，与所选 provider 不兼容")
  } else if status == StatusCode::TOO_MANY_REQUESTS {
    "外部 provider 已限流，请稍后重试".into()
  } else {
    format!("{action}失败：服务返回 HTTP {}", status.as_u16())
  }
}

pub fn analyze(settings: &ProviderSettings, instructions: &str, frames: &[EvidenceImage]) -> Result<String, String> {
  settings.require_configured()?;
  let key = provider_config::api_key()?;
  analyze_with_key(settings, instructions, frames, key.as_deref())
}

pub(crate) fn analyze_with_key(
  settings: &ProviderSettings,
  instructions: &str,
  frames: &[EvidenceImage],
  api_key: Option<&str>,
) -> Result<String, String> {
  if instructions.trim().is_empty() {
    return Err("外部分析指令不能为空".into());
  }
  if frames.is_empty() || frames.iter().any(|frame| frame.timestamp_ms < 0 || frame.png_base64.is_empty()) {
    return Err("外部分析需要带非负真实时间戳和 PNG 数据的证据帧".into());
  }
  match settings.kind {
    ProviderKind::OpenAiCompatible => analyze_openai(settings, instructions, frames, api_key),
    ProviderKind::Ollama => analyze_ollama(settings, instructions, frames, api_key),
  }
}

fn analyze_openai(settings: &ProviderSettings, instructions: &str, frames: &[EvidenceImage], key: Option<&str>) -> Result<String, String> {
  let client = client(settings)?;
  let url = operation_url(settings, Operation::Chat)?;
  let first = send(authorized(client.post(url.clone()).json(&openai_body(settings, instructions, frames, true)), key))?;
  if first.status().is_success() {
    return success_text(first, "/chat/completions");
  }
  let status = first.status();
  let error_body = first.text().map_err(|_| "读取外部 provider 错误响应失败".to_string())?;
  if response_format_unsupported(status, &error_body) {
    log::warn!("OpenAI 兼容服务明确不支持 response_format，本次降级重试且不再附带该参数");
    let retry = send(authorized(client.post(url).json(&openai_body(settings, instructions, frames, false)), key))?;
    return success_text(retry, "/chat/completions");
  }
  Err(status_message(status, "外部分析", "/chat/completions"))
}

fn analyze_ollama(settings: &ProviderSettings, instructions: &str, frames: &[EvidenceImage], key: Option<&str>) -> Result<String, String> {
  let images = frames.iter().map(|frame| frame.png_base64.as_str()).collect::<Vec<_>>();
  let body = json!({
    "model": settings.model.trim(),
    "messages": [{"role":"user","content":instructions,"images":images}],
    "stream": false,
    "format": "json"
  });
  let response = send(authorized(client(settings)?.post(operation_url(settings, Operation::Chat)?).json(&body), key))?;
  success_text(response, "/api/chat")
}

fn openai_body(settings: &ProviderSettings, instructions: &str, frames: &[EvidenceImage], with_format: bool) -> Value {
  let mut content = vec![json!({"type":"text","text":instructions})];
  content.extend(frames.iter().map(|frame| {
    json!({"type":"image_url","image_url":{"url":format!("data:image/png;base64,{}",frame.png_base64)}})
  }));
  let mut body = json!({
    "model": settings.model.trim(),
    "messages": [{"role":"user","content":content}],
    "stream": false
  });
  if with_format {
    body["response_format"] = json!({"type":"json_object"});
  }
  body
}

fn response_format_unsupported(status: StatusCode, body: &str) -> bool {
  if !matches!(status, StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY) {
    return false;
  }
  let body = body.to_lowercase();
  let mentions = body.contains("response_format") || body.contains("response format");
  let rejected = ["unsupported", "not support", "unknown", "unrecognized", "unexpected", "invalid", "not permitted", "extra input", "不支持"]
    .iter()
    .any(|term| body.contains(term));
  mentions && rejected
}

fn success_text(response: Response, chat_path: &str) -> Result<String, String> {
  let status = response.status();
  if !status.is_success() {
    return Err(status_message(status, "外部分析", chat_path));
  }
  response.text().map_err(|_| "读取外部 provider 响应失败".into())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{BufRead, BufReader, Read, Write};
  use std::net::TcpListener;
  use std::sync::mpsc::{self, Receiver};
  use std::thread::{self, JoinHandle};

  fn test_settings(kind: ProviderKind, endpoint: &str) -> ProviderSettings {
    ProviderSettings { kind, endpoint: endpoint.into(), model: "m".into(), timeout_secs: 5 }
  }

  fn frames() -> Vec<EvidenceImage> {
    vec![
      EvidenceImage { timestamp_ms: 0, source_frame: Some(0), png_base64: "AA".into() },
      EvidenceImage { timestamp_ms: 700, source_frame: Some(17), png_base64: "BB".into() },
    ]
  }

  fn serve(responses: Vec<(&'static str, &'static str)>) -> (String, Receiver<String>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
      for (status, body) in responses {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let (mut raw, mut length) = (String::new(), 0);
        loop {
          let mut line = String::new();
          reader.read_line(&mut line).unwrap();
          if line.to_ascii_lowercase().starts_with("content-length:") {
            length = line.split(':').nth(1).unwrap().trim().parse().unwrap();
          }
          let done = matches!(line.as_str(), "\r\n" | "\n");
          raw.push_str(&line);
          if done { break; }
        }
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes).unwrap();
        raw.push_str(std::str::from_utf8(&bytes).unwrap());
        let _ = tx.send(raw);
        let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        stream.write_all(response.as_bytes()).unwrap();
      }
    });
    (endpoint, rx, handle)
  }

  fn request_body(request: &str) -> Value {
    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
  }

  #[test]
  fn normalizes_openai_and_ollama_base_urls() {
    let openai = test_settings(ProviderKind::OpenAiCompatible, "https://api.openai.com/v1/");
    assert_eq!(operation_url(&openai, Operation::Models).unwrap(), "https://api.openai.com/v1/models");
    assert_eq!(operation_url(&openai, Operation::Chat).unwrap(), "https://api.openai.com/v1/chat/completions");
    let full = test_settings(ProviderKind::OpenAiCompatible, "https://host/v1/chat/completions");
    assert_eq!(operation_url(&full, Operation::Models).unwrap(), "https://host/v1/models");
    let ollama = test_settings(ProviderKind::Ollama, "http://localhost:11434");
    assert_eq!(operation_url(&ollama, Operation::Models).unwrap(), "http://localhost:11434/api/tags");
    assert_eq!(operation_url(&ollama, Operation::Chat).unwrap(), "http://localhost:11434/api/chat");
    let api = test_settings(ProviderKind::Ollama, "http://localhost:11434/api/chat");
    assert_eq!(operation_url(&api, Operation::Models).unwrap(), "http://localhost:11434/api/tags");
  }

  #[test]
  fn openai_request_uses_parts_data_urls_and_optional_bearer() {
    let (base, rx, handle) = serve(vec![("200 OK", "{\"choices\":[]}")]);
    let settings = test_settings(ProviderKind::OpenAiCompatible, &format!("{base}/v1/"));
    analyze_with_key(&settings, "instructions", &frames(), Some("test-key")).unwrap();
    let request = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(request.starts_with("POST /v1/chat/completions"));
    assert!(request.to_ascii_lowercase().contains("authorization: bearer test-key"));
    let body = request_body(&request);
    assert_eq!(body["stream"], false);
    assert_eq!(body["response_format"]["type"], "json_object");
    assert_eq!(body["messages"][0]["content"][0]["text"], "instructions");
    assert_eq!(body["messages"][0]["content"][1]["image_url"]["url"], "data:image/png;base64,AA");
    handle.join().unwrap();
  }

  #[test]
  fn ollama_request_uses_raw_base64_images_and_json_format() {
    let (base, rx, handle) = serve(vec![("200 OK", "{\"message\":{}}")]);
    let settings = test_settings(ProviderKind::Ollama, &base);
    analyze_with_key(&settings, "instructions", &frames(), None).unwrap();
    let request = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(request.starts_with("POST /api/chat"));
    assert!(!request.to_ascii_lowercase().contains("authorization:"));
    let body = request_body(&request);
    assert_eq!(body["format"], "json");
    assert_eq!(body["stream"], false);
    assert_eq!(body["messages"][0]["content"], "instructions");
    assert_eq!(body["messages"][0]["images"], serde_json::json!(["AA", "BB"]));
    assert!(body.get("response_format").is_none());
    handle.join().unwrap();
  }

  #[test]
  fn unsupported_response_format_retries_exactly_once() {
    let responses = vec![
      ("400 Bad Request", "{\"error\":\"response_format is unsupported\"}"),
      ("200 OK", "{\"choices\":[]}"),
    ];
    let (base, rx, handle) = serve(responses);
    let settings = test_settings(ProviderKind::OpenAiCompatible, &format!("{base}/v1"));
    analyze_with_key(&settings, "instructions", &frames(), None).unwrap();
    let first = request_body(&rx.recv_timeout(Duration::from_secs(5)).unwrap());
    let retry = request_body(&rx.recv_timeout(Duration::from_secs(5)).unwrap());
    assert!(first.get("response_format").is_some());
    assert!(retry.get("response_format").is_none());
    handle.join().unwrap();
  }

  #[test]
  fn connection_reports_missing_models_auth_and_incompatible_services() {
    let (base, rx, handle) = serve(vec![("200 OK", "{\"data\":[{\"id\":\"m\"}]}")]);
    let settings = test_settings(ProviderKind::OpenAiCompatible, &format!("{base}/v1"));
    assert!(test_connection_with_key(&settings, None).unwrap().ok);
    assert!(rx.recv_timeout(Duration::from_secs(5)).unwrap().starts_with("GET /v1/models"));
    handle.join().unwrap();

    let (base, rx, handle) = serve(vec![("200 OK", "{\"models\":[{\"name\":\"m:latest\"}]}")]);
    let settings = test_settings(ProviderKind::Ollama, &base);
    assert!(test_connection_with_key(&settings, None).unwrap().ok);
    assert!(rx.recv_timeout(Duration::from_secs(5)).unwrap().starts_with("GET /api/tags"));
    handle.join().unwrap();

    let (base, _, handle) = serve(vec![("200 OK", "{\"data\":[]}")]);
    let settings = test_settings(ProviderKind::OpenAiCompatible, &base);
    let result = test_connection_with_key(&settings, None).unwrap();
    assert!(!result.ok && result.message.contains("未找到"));
    handle.join().unwrap();

    let (base, _, handle) = serve(vec![("401 Unauthorized", "{}")]);
    let settings = test_settings(ProviderKind::Ollama, &base);
    let result = test_connection_with_key(&settings, None).unwrap();
    assert!(!result.ok && result.message.contains("鉴权失败"));
    handle.join().unwrap();

    let (base, _, handle) = serve(vec![("404 Not Found", "{}")]);
    let settings = test_settings(ProviderKind::Ollama, &base);
    let result = test_connection_with_key(&settings, None).unwrap();
    assert!(!result.ok && result.message.contains("不兼容"));
    handle.join().unwrap();
  }
}
