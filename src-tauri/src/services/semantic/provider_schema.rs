use serde_json::Value;

use super::provider_http::EvidenceImage;
use super::provider_types::ProviderEvent;

pub fn prompt(frames: &[EvidenceImage]) -> String {
  let timeline = frames
    .iter()
    .enumerate()
    .map(|(index, frame)| format!("帧 {index}：{} ms", frame.timestamp_ms))
    .collect::<Vec<_>>()
    .join("\n");
  format!(
    "你是视频证据分析器。输入仅是带真实毫秒时间戳的稀疏采样图片，不是完整视频；采样间隔内不可见，静态画面也不能证明动作或因果。\n\
    只按证据描述可见细节、详细动作和可直接观察的连续事件，禁止编造没看到的事件，禁止把先后关系等同因果。候选因果必须标为 hypothesis；可观察连续事件标为 observed。不要用确定语气包装推测。\n\
    仅输出一个 JSON 对象，不要 Markdown：{{\"description\":\"整体描述（非空字符串）\",\"events\":[{{\"text\":\"事件描述\",\"startMs\":0,\"endMs\":1000,\"certainty\":\"observed\",\"evidence\":[0,1]}}]}}。\n\
    时间使用原视频毫秒时间轴且 startMs<=endMs；evidence 必须是非空的输入帧索引数组（从 0 开始），事件区间必须覆盖至少一个证据帧时间戳。无法确定时给保守区间，不得虚构精度。\n\
    输入帧索引与时间戳：\n{timeline}"
  )
}

pub fn parse_response(
  content: &str,
  frame_count: usize,
  frame_times: &[i64],
  duration_ms: i64,
) -> Result<(String, Vec<ProviderEvent>), String> {
  validate_frames(frame_count, frame_times, duration_ms)?;
  let document = document_value(content)?;
  let description = document
    .get("description")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|text| !text.is_empty())
    .ok_or_else(|| "provider 返回 JSON 缺少非空 description 字符串".to_string())?
    .to_string();
  let events = document
    .get("events")
    .and_then(Value::as_array)
    .ok_or_else(|| "provider 返回 JSON 缺少 events 数组".to_string())?;
  let events = events
    .iter()
    .enumerate()
    .map(|(index, event)| parse_event(event, index, frame_count, frame_times, duration_ms))
    .collect::<Result<Vec<_>, _>>()?;
  Ok((description, events))
}

fn validate_frames(frame_count: usize, frame_times: &[i64], duration_ms: i64) -> Result<(), String> {
  if frame_count == 0 || frame_times.len() != frame_count {
    return Err("证据帧数量与 frameTimes 长度不一致或为空".into());
  }
  if duration_ms < 0 {
    return Err("视频时长不能为负数".into());
  }
  if frame_times.iter().any(|&time| time < 0 || time > duration_ms) {
    return Err("证据帧时间戳必须非负且不超过视频时长".into());
  }
  Ok(())
}

fn document_value(content: &str) -> Result<Value, String> {
  let text = strip_fence(content.trim());
  let value: Value = serde_json::from_str(text).map_err(|_| "provider 返回内容不是有效 JSON".to_string())?;
  if let Some(content) = value.pointer("/choices/0/message/content") {
    return document_value(&content_text(content)?);
  }
  if let Some(content) = value.get("message").and_then(|message| message.get("content")) {
    return document_value(&content_text(content)?);
  }
  Ok(value)
}

fn content_text(content: &Value) -> Result<String, String> {
  if let Some(text) = content.as_str() {
    return Ok(text.to_string());
  }
  let parts = content.as_array().ok_or_else(|| "provider 的 message.content 不是字符串或 text parts 数组".to_string())?;
  let mut output = String::new();
  for part in parts {
    if part.get("type").and_then(Value::as_str) == Some("text") {
      output.push_str(part.get("text").and_then(Value::as_str).ok_or_else(|| "text part 缺少 text 字符串".to_string())?);
    }
  }
  if output.trim().is_empty() {
    return Err("provider 的 text parts 中没有文本".into());
  }
  Ok(output)
}

fn strip_fence(text: &str) -> &str {
  let Some(rest) = text.strip_prefix("```") else { return text };
  let Some((language, body)) = rest.split_once('\n') else { return text };
  if !matches!(language.trim(), "" | "json" | "JSON") {
    return text;
  }
  let Some(end) = body.rfind("```") else { return text };
  if body[end + 3..].trim().is_empty() { body[..end].trim() } else { text }
}

fn parse_event(
  event: &Value,
  index: usize,
  frame_count: usize,
  frame_times: &[i64],
  duration_ms: i64,
) -> Result<ProviderEvent, String> {
  let text = event
    .get("text")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|text| !text.is_empty())
    .ok_or_else(|| format!("事件 {} 缺少非空 text", index))?
    .to_string();
  let start_ms = event_time(event, "startMs", index)?;
  let end_ms = event_time(event, "endMs", index)?;
  if start_ms > end_ms || end_ms > duration_ms {
    return Err(format!("事件 {index} 时间范围无效：须 startMs<=endMs 且 endMs 不超过视频时长"));
  }
  let certainty = event.get("certainty").and_then(Value::as_str).unwrap_or_default().to_string();
  if !matches!(certainty.as_str(), "observed" | "hypothesis") {
    return Err(format!("事件 {index} 的 certainty 仅允许 observed 或 hypothesis"));
  }
  let raw = event.get("evidence").and_then(Value::as_array).ok_or_else(|| format!("事件 {index} 缺少 evidence 数组"))?;
  if raw.is_empty() {
    return Err(format!("事件 {index} 的 evidence 不能为空"));
  }
  let evidence = raw
    .iter()
    .map(|value| {
      let frame = value.as_u64().and_then(|value| usize::try_from(value).ok()).filter(|&frame| frame < frame_count);
      frame.ok_or_else(|| format!("事件 {index} 的 evidence 包含越界或非整数帧索引"))
    })
    .collect::<Result<Vec<_>, _>>()?;
  if !evidence.iter().any(|&frame| start_ms <= frame_times[frame] && frame_times[frame] <= end_ms) {
    return Err(format!("事件 {index} 的时间区间未覆盖任何证据帧时间戳"));
  }
  Ok(ProviderEvent { text, start_ms, end_ms, certainty, evidence })
}

fn event_time(event: &Value, field: &str, index: usize) -> Result<i64, String> {
  let value = event.get(field).and_then(Value::as_f64).ok_or_else(|| format!("事件 {index} 的 {field} 必须是毫秒数值"))?;
  if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > i64::MAX as f64 {
    return Err(format!("事件 {index} 的 {field} 必须是有限、非负的整数毫秒"));
  }
  Ok(value as i64)
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn event(certainty: &str) -> Value {
    json!({"text":"人物起身","startMs":100,"endMs":900,"certainty":certainty,"evidence":[0,2]})
  }

  fn document(event: Value) -> String {
    json!({"description":"多帧中的人物变化","events":[event]}).to_string()
  }

  #[test]
  fn prompt_contains_sparse_limits_and_real_frame_times() {
    let frames = vec![
      EvidenceImage { timestamp_ms: 0, source_frame: Some(0), png_base64: "a".into() },
      EvidenceImage { timestamp_ms: 1750, source_frame: Some(42), png_base64: "b".into() },
    ];
    let prompt = prompt(&frames);
    assert!(prompt.contains("稀疏采样") && prompt.contains("帧 1：1750 ms"));
    assert!(prompt.contains("先后关系等同因果") && prompt.contains("hypothesis"));
    assert!(prompt.contains("observed") && prompt.contains("description"));
  }

  #[test]
  fn parses_fenced_json_with_irregular_noncontiguous_evidence() {
    let content = format!("```json\n{}\n```", document(event("observed")));
    let (description, events) = parse_response(&content, 3, &[100, 950, 1000], 4000).unwrap();
    assert_eq!(description, "多帧中的人物变化");
    assert_eq!(events[0].evidence, [0, 2]);
    assert_eq!(events[0].start_ms, 100);
  }

  #[test]
  fn accepts_openai_string_parts_and_ollama_envelopes() {
    let doc = document(event("hypothesis"));
    let openai = json!({"choices":[{"message":{"content":doc}}]}).to_string();
    let parts = json!({"choices":[{"message":{"content":[{"type":"text","text":doc}]}}]}).to_string();
    let ollama = json!({"message":{"content":doc}}).to_string();
    for content in [openai, parts, ollama] {
      let (_, events) = parse_response(&content, 3, &[100, 950, 1000], 4000).unwrap();
      assert_eq!(events[0].certainty, "hypothesis");
    }
  }

  #[test]
  fn rejects_bad_time_certainty_and_uncovered_evidence() {
    let mut bad = Vec::new();
    for (start, end) in [(-1, 900), (900, 100), (100, 4001)] {
      let mut value = event("observed");
      value["startMs"] = json!(start);
      value["endMs"] = json!(end);
      bad.push(value);
    }
    for evidence in [json!([]), json!([3]), json!([1])] {
      let mut value = event("observed");
      value["evidence"] = evidence;
      bad.push(value);
    }
    bad.push(event("certain"));
    for value in bad {
      assert!(parse_response(&document(value), 3, &[100, 950, 1000], 4000).is_err());
    }
    assert!(parse_response(&json!({"description":" ","events":[]}).to_string(), 1, &[0], 0).is_err());
    assert!(parse_response(&document(event("observed")), 2, &[100, 950, 1000], 4000).is_err());
  }
}
