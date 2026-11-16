//! SillyTavern 월드인포(로어북) → WorldInfoEntryInput
//!
//! 두 가지 형식을 모두 받는다.
//! - ST 월드 파일: `{ name?, entries: { "0": { key, keysecondary, content, constant, selective, order, disable } } }`
//! - 캐릭터 카드 V2/V3의 character_book: `{ name?, entries: [ { keys, secondary_keys, content, enabled,
//!   insertion_order, priority, constant, selective } ] }`
use serde_json::Value;

use crate::models::conversation::WorldInfoEntryInput;

use super::{as_str, str_list};

/// 항목들 변환. book_id는 저장할 때 채운다(빈 문자열로 둠).
pub fn map_entries(book: &Value, warnings: &mut Vec<String>) -> Vec<WorldInfoEntryInput> {
    let entries: Vec<&Value> = match &book["entries"] {
        Value::Array(a) => a.iter().collect(),
        // ST 월드 파일은 uid를 키로 하는 객체. 키(숫자) 순서대로 정렬
        Value::Object(m) => {
            let mut items: Vec<(&String, &Value)> = m.iter().collect();
            items.sort_by_key(|(k, _)| k.parse::<i64>().unwrap_or(i64::MAX));
            items.into_iter().map(|(_, v)| v).collect()
        }
        _ => vec![],
    };

    let mut out = vec![];
    for e in entries {
        let content = as_str(e, "content").unwrap_or_default().trim().to_string();
        if content.is_empty() {
            continue;
        }
        let keys = first_list(e, &["keys", "key"]);
        let constant = e["constant"].as_bool().unwrap_or(false);
        if keys.is_empty() && !constant {
            warnings.push(format!(
                "월드인포 '{}': 키워드가 없고 상시 항목도 아니라서 끈 상태로 가져옴",
                as_str(e, "comment").or(as_str(e, "name")).unwrap_or("이름 없음")
            ));
        }
        // selective가 꺼져 있으면 보조 키를 쓰지 않는다 (우리 매처는 보조 키가 있으면 필수 조건으로 봄)
        let selective = e["selective"].as_bool().unwrap_or(false);
        let secondary = if selective {
            first_list(e, &["secondary_keys", "keysecondary"])
        } else {
            vec![]
        };
        let enabled = match (e["enabled"].as_bool(), e["disable"].as_bool()) {
            (Some(en), _) => en,
            (None, Some(dis)) => !dis,
            _ => true,
        } && (constant || !keys.is_empty());
        let priority = ["insertion_order", "order", "priority"].iter().find_map(|k| e[*k].as_i64()).unwrap_or(100);

        out.push(WorldInfoEntryInput {
            comment: as_str(e, "comment").or(as_str(e, "name")).unwrap_or_default().trim().to_string(),
            id: None,
            book_id: String::new(),
            keys,
            secondary_keys: secondary,
            content,
            enabled,
            priority,
            constant,
        });
    }
    out
}

fn first_list(v: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .map(|k| str_list(&v[*k]))
        .find(|l| !l.is_empty())
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn st_world_file_format() {
        let mut w = vec![];
        let entries = map_entries(
            &json!({ "entries": {
                "10": { "key": ["castle"], "keysecondary": ["gate"], "selective": true, "content": "B", "order": 5 },
                "2": { "key": ["dragon"], "keysecondary": ["x"], "selective": false, "content": "A", "disable": true },
                "3": { "key": [], "constant": true, "content": "C" },
                "4": { "key": [], "content": "D", "comment": "orphan" },
                "5": { "key": ["e"], "content": "   " }
            }}),
            &mut w,
        );
        let contents: Vec<_> = entries.iter().map(|e| e.content.as_str()).collect();
        assert_eq!(contents, vec!["A", "C", "D", "B"]);
        assert!(!entries[0].enabled);
        assert!(entries[0].secondary_keys.is_empty(), "selective=false면 보조 키 무시");
        assert!(entries[1].constant && entries[1].enabled);
        assert!(!entries[2].enabled);
        assert_eq!(entries[3].secondary_keys, vec!["gate"]);
        assert_eq!(entries[3].priority, 5);
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn card_book_format() {
        let mut w = vec![];
        let entries = map_entries(
            &json!({ "entries": [
                { "keys": ["elf"], "secondary_keys": [], "content": "Elves", "enabled": true, "insertion_order": 7 }
            ]}),
            &mut w,
        );
        assert_eq!(entries[0].keys, vec!["elf"]);
        assert_eq!(entries[0].priority, 7);
        assert!(w.is_empty());
    }
}
