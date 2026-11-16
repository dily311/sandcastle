//! AI 그림 파일에 들어 있는 생성 정보 읽기 (갤러리에 올린 그림).
//!
//! PNG 글 조각(tEXt, 압축하지 않은 iTXt)만 읽는다 — 그림 내용은 해석하지 않는다.
//! - NovelAI: `Comment`(JSON: prompt, uc, steps, scale, sampler, seed, width, height), `Software`, `Source`
//! - Stable Diffusion WebUI(A1111·Forge): `parameters` (프롬프트 / Negative prompt: … / Steps: 20, Sampler: …)
//! - ComfyUI: `prompt`·`workflow` JSON은 너무 커서 "있음"만 표시
//!
//! 알아본 항목은 보기 좋은 이름으로, 나머지 글 조각은 원래 이름으로 남긴다.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 화면에 보일 정보 한 줄
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct MetaItem {
    pub key: String,
    pub value: String,
}

/// 읽어 낸 생성 정보
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageMeta {
    pub prompt: String,
    pub items: Vec<MetaItem>,
}

const MAX_TEXT: usize = 64 * 1024;

/// PNG의 글 조각들 (키, 글)
pub fn png_text_chunks(bytes: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return out;
    }
    let mut i = 8;
    while i + 12 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]) as usize;
        let kind = &bytes[i + 4..i + 8];
        let start = i + 8;
        let Some(end) = start.checked_add(len).filter(|e| *e + 4 <= bytes.len()) else { break };
        let data = &bytes[start..end];
        match kind {
            b"tEXt" => {
                if let Some(z) = data.iter().position(|b| *b == 0) {
                    let key = latin1(&data[..z]);
                    out.push((key, latin1(&data[z + 1..])));
                }
            }
            b"iTXt" => {
                // 키\0 압축여부 압축방식 언어\0 번역키\0 글
                if let Some(z) = data.iter().position(|b| *b == 0) {
                    let key = latin1(&data[..z]);
                    let rest = &data[z + 1..];
                    if rest.len() >= 2 && rest[0] == 0 {
                        let rest = &rest[2..];
                        if let Some(l) = rest.iter().position(|b| *b == 0) {
                            let rest = &rest[l + 1..];
                            if let Some(t) = rest.iter().position(|b| *b == 0) {
                                out.push((key, String::from_utf8_lossy(&rest[t + 1..]).into_owned()));
                            }
                        }
                    }
                }
            }
            b"IEND" => break,
            _ => {}
        }
        i = end + 4;
    }
    out.into_iter().map(|(k, v)| (k, v.chars().take(MAX_TEXT).collect())).collect()
}

fn latin1(b: &[u8]) -> String {
    // tEXt는 Latin-1이지만 실제로는 UTF-8을 넣는 프로그램이 많다
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => b.iter().map(|c| *c as char).collect(),
    }
}

fn push(items: &mut Vec<MetaItem>, key: &str, value: impl Into<String>) {
    let value = value.into();
    if !value.trim().is_empty() {
        items.push(MetaItem { key: key.to_string(), value: value.trim().to_string() });
    }
}

fn json_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// NovelAI Comment JSON
fn novelai(json: &Value, items: &mut Vec<MetaItem>) -> String {
    let prompt = json_str(&json["prompt"]);
    push(items, "네거티브", json_str(&json["uc"]));
    for (k, label) in [("steps", "단계"), ("scale", "프롬프트 반영"), ("sampler", "샘플러"), ("seed", "시드"), ("noise_schedule", "노이즈 일정"), ("cfg_rescale", "CFG 리스케일")] {
        push(items, label, json_str(&json[k]));
    }
    if let (Some(w), Some(h)) = (json["width"].as_u64(), json["height"].as_u64()) {
        push(items, "크기", format!("{w}x{h}"));
    }
    prompt
}

/// A1111 parameters 글
fn a1111(text: &str, items: &mut Vec<MetaItem>) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let settings_at = lines.iter().rposition(|l| l.starts_with("Steps:")).unwrap_or(lines.len());
    let neg_at = lines[..settings_at].iter().position(|l| l.starts_with("Negative prompt:"));
    let prompt = lines[..neg_at.unwrap_or(settings_at)].join("\n");
    if let Some(n) = neg_at {
        let neg = lines[n..settings_at].join("\n");
        push(items, "네거티브", neg.trim_start_matches("Negative prompt:"));
    }
    if let Some(line) = lines.get(settings_at) {
        // "Steps: 20, Sampler: Euler a, CFG scale: 7, Seed: 1, Size: 512x768, Model: x" (값에 쉼표가 든 따옴표 묶음은 드묾)
        for part in line.split(", ") {
            if let Some((k, v)) = part.split_once(": ") {
                let label = match k {
                    "Steps" => "단계",
                    "Sampler" => "샘플러",
                    "CFG scale" => "프롬프트 반영",
                    "Seed" => "시드",
                    "Size" => "크기",
                    "Model" => "모델",
                    other => other,
                };
                push(items, label, v);
            }
        }
    }
    prompt
}

/// 글 조각들 → 생성 정보
pub fn parse(chunks: &[(String, String)]) -> ImageMeta {
    let mut items = Vec::new();
    let mut prompt = String::new();
    for (k, v) in chunks {
        match k.as_str() {
            "Comment" => match serde_json::from_str::<Value>(v) {
                Ok(json) if json.is_object() => {
                    let p = novelai(&json, &mut items);
                    if prompt.is_empty() {
                        prompt = p;
                    }
                }
                _ => push(&mut items, "Comment", v.clone()),
            },
            "parameters" => {
                let p = a1111(v, &mut items);
                if prompt.is_empty() {
                    prompt = p;
                }
            }
            "Description" if prompt.is_empty() => prompt = v.clone(),
            "Software" => push(&mut items, "프로그램", v.clone()),
            "Source" => push(&mut items, "모델", v.clone()),
            "prompt" | "workflow" => push(&mut items, if k == "prompt" { "ComfyUI 프롬프트" } else { "ComfyUI 워크플로" }, "있음"),
            "Title" | "Generation time" | "Description" => push(&mut items, k, v.clone()),
            other => push(&mut items, other, v.chars().take(500).collect::<String>()),
        }
    }
    ImageMeta { prompt: prompt.trim().to_string(), items }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(kind: &[u8], data: &[u8]) -> Vec<u8> {
        let mut c = (data.len() as u32).to_be_bytes().to_vec();
        c.extend_from_slice(kind);
        c.extend_from_slice(data);
        c.extend_from_slice(&[0, 0, 0, 0]); // crc는 읽지 않음
        c
    }
    fn png(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut b = b"\x89PNG\r\n\x1a\n".to_vec();
        for c in chunks {
            b.extend_from_slice(c);
        }
        b.extend(chunk(b"IEND", b""));
        b
    }

    #[test]
    fn reads_novelai_comment_and_software() {
        let comment = r#"{"prompt":"1girl, silver hair","uc":"lowres","steps":28,"scale":5.0,"sampler":"k_euler_ancestral","seed":42,"width":832,"height":1216}"#;
        let bytes = png(&[chunk(b"tEXt", b"Software\0NovelAI"), chunk(b"tEXt", format!("Comment\0{comment}").as_bytes())]);
        let meta = parse(&png_text_chunks(&bytes));
        assert_eq!(meta.prompt, "1girl, silver hair");
        let get = |k: &str| meta.items.iter().find(|i| i.key == k).map(|i| i.value.clone());
        assert_eq!(get("프로그램").as_deref(), Some("NovelAI"));
        assert_eq!(get("네거티브").as_deref(), Some("lowres"));
        assert_eq!(get("단계").as_deref(), Some("28"));
        assert_eq!(get("크기").as_deref(), Some("832x1216"));
    }

    #[test]
    fn reads_a1111_parameters_from_itxt() {
        let text = "a cat, sitting\nNegative prompt: blurry, dog\nSteps: 20, Sampler: Euler a, CFG scale: 7, Seed: 1, Size: 512x768, Model: anything";
        let mut data = b"parameters\0\0\0\0\0".to_vec();
        data.extend_from_slice(text.as_bytes());
        let meta = parse(&png_text_chunks(&png(&[chunk(b"iTXt", &data)])));
        assert_eq!(meta.prompt, "a cat, sitting");
        let get = |k: &str| meta.items.iter().find(|i| i.key == k).map(|i| i.value.clone());
        assert_eq!(get("네거티브").as_deref(), Some("blurry, dog"));
        assert_eq!(get("샘플러").as_deref(), Some("Euler a"));
        assert_eq!(get("모델").as_deref(), Some("anything"));
    }

    #[test]
    fn not_png_or_broken_is_empty() {
        assert!(png_text_chunks(b"GIF89a....").is_empty());
        let mut b = png(&[]);
        b.truncate(10);
        assert!(png_text_chunks(&b).is_empty());
    }
}
