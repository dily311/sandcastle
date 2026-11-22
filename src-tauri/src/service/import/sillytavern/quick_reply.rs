//! SillyTavern 빠른 답장(QR) .json → 우리 세트 (5-E-20-5-3·5-E-20-6-1, ADR-025).
//! 세트 모양: { name, disableSend, qrList: [{ label, title, message, isHidden, … }] }.
//! 버튼 하나 모양(.qr.json): { label, title, message, isHidden, … } — 여러 개를 고르면 세트 하나로 묶는다.
//! 자동 실행(executeOnStartup·executeOnUser·executeOnAi …)은 가져오지 않는다 — 스크립트는 사용자가 누를 때만 돈다.
use serde_json::Value;

use crate::{
    error::{AppError, Result},
    models::{quick_reply::QuickReplySetInput, settings::QuickReply},
};

/// 버튼 하나 모양인가 (.qr.json)
pub fn is_single_qr(json: &Value) -> bool {
    json.get("qrList").is_none() && json.get("message").is_some_and(Value::is_string)
}

/// 버튼 하나(.qr.json) → 버튼. 자동 실행 설정이 있으면 경고
pub fn map_qr_item(q: &Value, warnings: &mut Vec<String>) -> Option<QuickReply> {
    let message = q.get("message").and_then(Value::as_str).unwrap_or_default().to_string();
    let label = [q.get("label"), q.get("title")]
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .find(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| message.chars().take(20).collect());
    if label.is_empty() && message.trim().is_empty() {
        return None;
    }
    if AUTO_KEYS.iter().any(|k| q.get(*k).and_then(Value::as_bool) == Some(true)) {
        warnings.push(format!("'{label}': 자동 실행 설정은 가져오지 않음 — 누를 때만 실행됨"));
    }
    Some(QuickReply {
        id: uuid::Uuid::new_v4().to_string(),
        label,
        message,
        hidden: q.get("isHidden").and_then(Value::as_bool).unwrap_or(false),
    })
}

const AUTO_KEYS: [&str; 7] = [
    "executeOnStartup",
    "executeOnUser",
    "executeOnAi",
    "executeOnChatChange",
    "executeOnGroupMemberDraft",
    "executeOnNewChat",
    "executeBeforeGeneration",
];

pub fn map_qr_set(json: &Value, fallback_name: &str, warnings: &mut Vec<String>) -> Result<QuickReplySetInput> {
    let list = json
        .get("qrList")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::validation("SillyTavern 빠른 답장 세트가 아닙니다 (qrList가 없음)", None))?;
    let mut items = vec![];
    let mut auto = 0;
    for q in list {
        let message = q.get("message").and_then(Value::as_str).unwrap_or_default().to_string();
        let label = [q.get("label"), q.get("title")]
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::trim)
            .find(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| message.chars().take(20).collect());
        if label.is_empty() && message.trim().is_empty() {
            continue;
        }
        if AUTO_KEYS.iter().any(|k| q.get(*k).and_then(Value::as_bool) == Some(true)) {
            auto += 1;
        }
        items.push(QuickReply {
            id: uuid::Uuid::new_v4().to_string(),
            label,
            message,
            hidden: q.get("isHidden").and_then(Value::as_bool).unwrap_or(false),
        });
    }
    if auto > 0 {
        warnings.push(format!("자동 실행 설정 {auto}개는 가져오지 않음 — 누를 때만 실행됨"));
    }
    let name = json
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_name)
        .to_string();
    Ok(QuickReplySetInput {
        id: None,
        name,
        scope: "global".into(),
        scope_id: None,
        sort_order: 0,
        enabled: true,
        auto_send: !json.get("disableSend").and_then(Value::as_bool).unwrap_or(false),
        items,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn st_quick_reply_set_maps_labels_messages_and_skips_auto_run() {
        let mut w = vec![];
        let set = super::map_qr_set(
            &json!({
                "version": 2, "name": "지시", "disableSend": true,
                "qrList": [
                    { "label": "빠른 지시", "message": "/input 내용? | /gen {{pipe}}", "isHidden": false, "executeOnAi": true },
                    { "label": "", "title": "인사", "message": "안녕!" },
                    { "label": "", "message": "" }
                ]
            }),
            "파일",
            &mut w,
        )
        .unwrap();
        assert_eq!(set.name, "지시");
        assert!(!set.auto_send, "disableSend → 입력칸에 넣기만");
        assert_eq!(
            set.items.iter().map(|i| (i.label.as_str(), i.message.as_str())).collect::<Vec<_>>(),
            vec![("빠른 지시", "/input 내용? | /gen {{pipe}}"), ("인사", "안녕!")]
        );
        assert_eq!(w.len(), 1, "자동 실행은 가져오지 않음");
        assert!(super::map_qr_set(&json!({ "name": "x" }), "f", &mut w).is_err());
    }

    #[test]
    fn real_single_qr_files_become_buttons() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sillyTavern/qr");
        let mut labels = vec![];
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let v: serde_json::Value = serde_json::from_slice(&std::fs::read(e.path()).unwrap()).unwrap();
            assert!(super::is_single_qr(&v), "{:?}", e.path());
            let mut w = vec![];
            let q = super::map_qr_item(&v, &mut w).expect("버튼");
            assert!(q.message.starts_with('/'), "스크립트");
            assert!(w.is_empty(), "자동 실행 없음: {w:?}");
            labels.push(q.label);
        }
        labels.sort();
        assert_eq!(labels.len(), 4);
        assert!(labels.contains(&"무물🎤".to_string()));
    }
}
