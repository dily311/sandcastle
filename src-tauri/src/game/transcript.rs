//! 대사 기록(transcript): 이어하기/불러오기 때 화면에 다시 보여줄 "이전에 나온 줄들".
//!
//! 별도 테이블을 만들지 않고 일지(game_journal)를 그대로 읽는다. 줄(StageLine)은 이미 그 자리에 저장돼 있다:
//!   - 씬 대본: SceneScript 이벤트의 payload (sceneId + lines)
//!   - 대화: kind=message 항목의 payload (role + lines). 유저 입력도 `player`의 한 줄로 같이 들어간다.
//! 일지는 세션 단위 seq로 정렬되고 분기(fork) 때 seq까지 복사되므로, 기록도 책갈피/분기와 자동으로 맞는다.
//! 연출만 있는 줄(Direction뿐인 항목)은 기록에서 뺀다 — 지금 화면의 모습은 state.stage가 복원한다.
//! 전체 재생(다시 보기)은 read_journal로 연출까지 모두 읽는다.
use crate::db::game::get_game_journal_lookup;
use crate::error::AppError;
use crate::game::runtime::{GameEvent, RenderState};
use crate::game::stage::{LineKind, StageLine};
use crate::models::game::GameJournal;
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TranscriptKind {
    SceneScript,
    Message,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptEntry {
    pub seq: u32,
    pub game_time: i64,
    pub kind: TranscriptKind,
    /// SceneScript일 때 어느 씬의 대본인지
    pub scene_id: Option<String>,
    /// Message일 때 "user" / "assistant"
    pub role: Option<String>,
    pub lines: Vec<StageLine>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptPage {
    /// 오래된 것 → 최신 순
    pub entries: Vec<TranscriptEntry>,
    /// 이 페이지보다 더 오래된 기록이 있는가
    pub has_more: bool,
    /// 이 페이지에서 가장 오래된 항목의 seq. 다음(더 오래된) 페이지를 읽을 때 beforeSeq로 넘긴다.
    pub oldest_seq: Option<u32>,
}

/// continue_game / load_save의 응답. events는 항상 비어 있고(새로 생긴 일이 없다), 화면은 state + history로 복원한다.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueGameResponse {
    pub events: Vec<GameEvent>,
    pub state: RenderState,
    pub history: TranscriptPage,
}

/// 일지 항목들에서 기록에 보일 항목만 뽑는다 (seq 오름차순). 줄을 읽을 수 없는 항목(옛 형식 등)은 건너뛴다.
pub fn entries_from_journal(rows: &[GameJournal]) -> Vec<TranscriptEntry> {
    let mut out = Vec::new();
    for r in rows {
        let payload = &r.payload_json;
        let (kind, scene_id, role) = match r.kind.as_str() {
            "event" if payload.get("type").and_then(|t| t.as_str()) == Some("sceneScript") => {
                (TranscriptKind::SceneScript, payload.get("sceneId").and_then(|v| v.as_str()).map(str::to_string), None)
            }
            "message" => (TranscriptKind::Message, None, payload.get("role").and_then(|v| v.as_str()).map(str::to_string)),
            _ => continue,
        };
        let Some(lines) = payload.get("lines").and_then(|l| serde_json::from_value::<Vec<StageLine>>(l.clone()).ok()) else {
            continue;
        };
        if !lines.iter().any(|l| l.kind != LineKind::Direction) {
            continue;
        }
        out.push(TranscriptEntry { seq: r.seq, game_time: r.game_time, kind, scene_id, role, lines });
    }
    out.sort_by_key(|e| e.seq);
    out
}

/// 최신 쪽에서 limit개의 기록을 읽는다. before_seq가 있으면 그 seq보다 오래된 것만 (이전 페이지 읽기).
/// 일지는 커맨드/이벤트가 섞여 있어서 seq 구간(1000개씩)을 뒤로 가며 모은다.
pub async fn transcript_page(pool: &SqlitePool, session_id: &str, head_seq: u32, before_seq: Option<u32>, limit: usize) -> Result<TranscriptPage, AppError> {
    const WINDOW: u32 = 1000;
    let mut to = match before_seq {
        Some(b) => b.saturating_sub(1),
        None => head_seq,
    };
    let mut newest_first: Vec<TranscriptEntry> = Vec::new();
    while newest_first.len() <= limit && to > 0 {
        let from = to.saturating_sub(WINDOW - 1).max(1);
        let rows = get_game_journal_lookup(pool, session_id, Some(from), to).await?;
        let mut entries = entries_from_journal(&rows);
        entries.reverse();
        newest_first.extend(entries);
        to = from.saturating_sub(1);
    }
    let has_more = newest_first.len() > limit || to > 0;
    newest_first.truncate(limit);
    newest_first.reverse();
    let oldest_seq = newest_first.first().map(|e| e.seq);
    Ok(TranscriptPage { entries: newest_first, has_more, oldest_seq })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(seq: u32, kind: &str, payload: serde_json::Value) -> GameJournal {
        GameJournal { session_id: "s".into(), seq, kind: kind.into(), payload_json: payload, game_time: 0, created_at: 0 }
    }
    fn line(text: &str) -> serde_json::Value {
        json!({ "kind": "dialogue", "speaker": "c1", "text": text, "cues": [] })
    }

    #[test]
    fn picks_scene_scripts_and_messages_only_with_text() {
        let rows = vec![
            row(3, "message", json!({ "role": "assistant", "lines": [line("안녕")] })),
            row(1, "event", json!({ "type": "sceneScript", "sceneId": "sc1", "lines": [line("대본")] })),
            row(2, "event", json!({ "type": "stageLines", "lines": [{ "kind": "direction", "speaker": null, "text": "", "cues": [] }] })),
            row(4, "command", json!({ "type": "moveTo" })),
            row(5, "message", json!({ "role": "user", "lines": [{ "kind": "direction", "speaker": null, "text": "", "cues": [] }] })), // 텍스트 없는 항목
            row(6, "message", json!({ "role": "user" })),                                                                       // 줄이 없는 옛 항목
        ];
        let e = entries_from_journal(&rows);
        assert_eq!(e.iter().map(|x| x.seq).collect::<Vec<_>>(), vec![1, 3]); // 오름차순, 텍스트가 있는 것만
        assert_eq!(e[0].kind, TranscriptKind::SceneScript);
        assert_eq!(e[0].scene_id.as_deref(), Some("sc1"));
        assert_eq!(e[1].role.as_deref(), Some("assistant"));
    }
}
