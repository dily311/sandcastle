//! SillyTavern 빠른 답장(QR) 세트 .json → 우리 세트 (5-E-20-5-3, ADR-025).
//! v2 모양: { name, disableSend, qrList: [{ label, title, message, isHidden, … }] }.
//! 자동 실행(executeOnStartup·executeOnUser·executeOnAi …)은 가져오지 않는다 — 스크립트는 사용자가 누를 때만 돈다.
use serde_json::Value;

use crate::{
    error::{AppError, Result},
    models::settings::{QuickReply, QuickReplySet},
};

pub fn map_qr_set(json: &Value, fallback_name: &str, warnings: &mut Vec<String>) -> Result<QuickReplySet> {
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
        if [
            "executeOnStartup",
            "executeOnUser",
            "executeOnAi",
            "executeOnChatChange",
            "executeOnGroupMemberDraft",
            "executeOnNewChat",
        ]
        .iter()
        .any(|k| q.get(*k).and_then(Value::as_bool) == Some(true))
        {
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
    Ok(QuickReplySet {
        id: uuid::Uuid::new_v4().to_string(),
        name,
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
}
