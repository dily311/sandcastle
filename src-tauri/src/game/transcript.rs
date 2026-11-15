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

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TranscriptKind {
    SceneScript,
    Message,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptEntry {
    pub seq: u32,
    pub game_time: i64,
    pub kind: TranscriptKind,
    /// SceneScript일 때 어느 씬의 대본인지
    pub scene_id: Option<String>,
    /// Message일 때 "user" / "assistant"
    pub role: Option<String>,
    /// Message일 때 어느 게임 대화의 메시지인지
    pub conversation_id: Option<String>,
    pub lines: Vec<StageLine>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
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
#[derive(Serialize, specta::Type)]
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
        let text = |k: &str| payload.get(k).and_then(|v| v.as_str()).map(str::to_string);
        let (kind, scene_id, role, conversation_id) = match r.kind.as_str() {
            "event" if payload.get("type").and_then(|t| t.as_str()) == Some("sceneScript") => (TranscriptKind::SceneScript, text("sceneId"), None, None),
            "message" => (TranscriptKind::Message, None, text("role"), text("conversationId")),
            _ => continue,
        };
        let Some(lines) = payload.get("lines").and_then(|l| serde_json::from_value::<Vec<StageLine>>(l.clone()).ok()) else {
            continue;
        };
        if !lines.iter().any(|l| l.kind != LineKind::Direction) {
            continue;
        }
        out.push(TranscriptEntry { seq: r.seq, game_time: r.game_time, kind, scene_id, role, conversation_id, lines });
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

/// "지금" 기록의 범위
#[derive(Debug, Clone)]
pub enum CurrentScope {
    /// 열린 씬: 이 씬이 마지막으로 열린 뒤의 대본 + 이 씬의 대화들(branch_key = 씬 id)
    Scene { scene_id: String, conversation_ids: Vec<String> },
    /// 씬 밖 1:1 대화: 이 NPC와 마지막으로 대화를 시작한 뒤의 메시지
    Talk { character_id: String, conversation_id: Option<String> },
}

/// 일지 항목이 범위의 시작점인가 (씬 대본이 열린 곳 / 대화를 시작한 곳)
fn is_anchor(row: &GameJournal, scope: &CurrentScope) -> bool {
    if row.kind != "event" {
        return false;
    }
    let p = &row.payload_json;
    let get = |k: &str| p.get(k).and_then(|v| v.as_str());
    match scope {
        CurrentScope::Scene { scene_id, .. } => get("type") == Some("sceneScript") && get("sceneId") == Some(scene_id.as_str()),
        CurrentScope::Talk { character_id, .. } => get("type") == Some("dialogueStarted") && get("gameCharacterId") == Some(character_id.as_str()),
    }
}

fn in_scope(e: &TranscriptEntry, scope: &CurrentScope) -> bool {
    match scope {
        CurrentScope::Scene { scene_id, conversation_ids } => match e.kind {
            TranscriptKind::SceneScript => e.scene_id.as_deref() == Some(scene_id.as_str()),
            TranscriptKind::Message => e.conversation_id.as_ref().is_some_and(|c| conversation_ids.contains(c)),
        },
        CurrentScope::Talk { conversation_id, .. } => e.kind == TranscriptKind::Message && e.conversation_id.is_some() && e.conversation_id == *conversation_id,
    }
}

/// 범위 안의 기록만 고른다 (순수 함수: 테스트용으로 분리). rows는 seq 오름차순이 아니어도 된다.
pub fn current_entries(rows: &[GameJournal], scope: &CurrentScope) -> Vec<TranscriptEntry> {
    let anchor = rows.iter().filter(|r| is_anchor(r, scope)).map(|r| r.seq).max().unwrap_or(0);
    let after: Vec<GameJournal> = rows.iter().filter(|r| r.seq >= anchor).cloned().collect();
    entries_from_journal(&after).into_iter().filter(|e| in_scope(e, scope)).collect()
}

/// 지금 씬·대화의 기록 (오래된 것 → 최신). 시작점을 찾을 때까지 일지를 뒤로 읽되, 너무 길면 최근 limit개만.
pub async fn current_page(pool: &SqlitePool, session_id: &str, head_seq: u32, scope: &CurrentScope, limit: usize) -> Result<TranscriptPage, AppError> {
    const WINDOW: u32 = 1000;
    const MAX_WINDOWS: u32 = 20;
    let mut rows: Vec<GameJournal> = Vec::new();
    let mut to = head_seq;
    for _ in 0..MAX_WINDOWS {
        if to == 0 {
            break;
        }
        let from = to.saturating_sub(WINDOW - 1).max(1);
        let chunk = get_game_journal_lookup(pool, session_id, Some(from), to).await?;
        let found = chunk.iter().any(|r| is_anchor(r, scope));
        rows.extend(chunk);
        to = from.saturating_sub(1);
        if found {
            break;
        }
    }
    let mut entries = current_entries(&rows, scope);
    let has_more = entries.len() > limit;
    if has_more {
        entries.drain(..entries.len() - limit);
    }
    let oldest_seq = entries.first().map(|e| e.seq);
    Ok(TranscriptPage { entries, has_more, oldest_seq })
}

/// 분기할 수 있는 시점 하나 (턴 끝 상태 스냅샷이 있는 seq)
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TurnPoint {
    pub seq: u32,
    pub game_time: i64,
    /// 그 턴에 무슨 일이 있었는지 한 줄 (마지막 대사, 없으면 한 행동)
    pub summary: String,
    pub scene_id: Option<String>,
    pub location_id: Option<String>,
}

fn command_label(payload: &serde_json::Value) -> Option<String> {
    let t = payload.get("type")?.as_str()?;
    Some(
        match t {
            "moveTo" => "이동",
            "interact" => "살펴보기",
            "talk" => "말 걸기",
            "stopTalk" => "대화 끝내기",
            "choose" => "선택",
            "endScene" => "장면 끝내기",
            "advanceTime" => "시간 보내기",
            "useItem" => "아이템 사용",
            "dropItem" => "아이템 버리기",
            other => other,
        }
        .to_string(),
    )
}

fn last_text(payload: &serde_json::Value) -> Option<String> {
    let lines = payload.get("lines")?.as_array()?;
    lines.iter().rev().find_map(|l| l.get("text").and_then(|t| t.as_str()).map(str::trim).filter(|t| !t.is_empty()).map(str::to_string))
}

/// 일지에서 턴 시점 목록을 만든다 (최신 → 오래된 순). 요약은 직전 시점 이후 항목들에서 고른다.
pub fn turn_points_from(rows: &[GameJournal]) -> Vec<TurnPoint> {
    let mut sorted: Vec<&GameJournal> = rows.iter().collect();
    sorted.sort_by_key(|r| r.seq);
    let mut out = Vec::new();
    let (mut text, mut action): (Option<String>, Option<String>) = (None, None);
    for r in sorted {
        match r.kind.as_str() {
            "message" => text = last_text(&r.payload_json).or(text),
            "event" if r.payload_json.get("type").and_then(|t| t.as_str()) == Some("sceneScript") => text = last_text(&r.payload_json).or(text),
            "command" => action = command_label(&r.payload_json).or(action),
            crate::game::journal::STATE_KIND => {
                let st = &r.payload_json;
                let get = |k: &str| st.get(k).and_then(|v| v.as_str()).map(str::to_string);
                let mut summary = text.take().or(action.take()).unwrap_or_else(|| "시작".into());
                if summary.chars().count() > 60 {
                    summary = summary.chars().take(60).collect::<String>() + "…";
                }
                action = None;
                out.push(TurnPoint { seq: r.seq, game_time: r.game_time, summary, scene_id: get("current_scene_id"), location_id: get("location_id") });
            }
            _ => {}
        }
    }
    out.reverse();
    out
}

/// 최근 limit개의 시점 (최신 → 오래된 순)
pub async fn turn_points(pool: &SqlitePool, session_id: &str, head_seq: u32, limit: usize) -> Result<Vec<TurnPoint>, AppError> {
    const WINDOW: u32 = 1000;
    let mut rows: Vec<GameJournal> = Vec::new();
    let mut to = head_seq;
    while to > 0 {
        let from = to.saturating_sub(WINDOW - 1).max(1);
        rows.extend(get_game_journal_lookup(pool, session_id, Some(from), to).await?);
        to = from.saturating_sub(1);
        if rows.iter().filter(|r| r.kind == crate::game::journal::STATE_KIND).count() > limit {
            break;
        }
    }
    let mut points = turn_points_from(&rows);
    points.truncate(limit);
    Ok(points)
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

    #[test]
    fn current_scene_only_since_it_last_opened() {
        let script = |seq, scene: &str, t: &str| row(seq, "event", json!({ "type": "sceneScript", "sceneId": scene, "lines": [line(t)] }));
        let msg = |seq, conv: &str, t: &str| row(seq, "message", json!({ "conversationId": conv, "role": "assistant", "lines": [line(t)] }));
        let rows = vec![
            script(1, "s1", "처음 열림"),
            msg(2, "c-s1", "옛 대화"),
            script(3, "s2", "다른 씬"),
            msg(4, "c-npc", "엘리안과 대화"),
            script(5, "s1", "다시 열림"),
            msg(6, "c-s1", "새 대화"),
            msg(7, "c-npc", "씬 밖 대화"),
        ];
        let scope = CurrentScope::Scene { scene_id: "s1".into(), conversation_ids: vec!["c-s1".into()] };
        let got: Vec<u32> = current_entries(&rows, &scope).iter().map(|e| e.seq).collect();
        assert_eq!(got, vec![5, 6]);
    }

    #[test]
    fn current_talk_only_since_dialogue_started() {
        let rows = vec![
            row(1, "event", json!({ "type": "dialogueStarted", "gameCharacterId": "c1" })),
            row(2, "message", json!({ "conversationId": "conv", "role": "assistant", "lines": [line("지난번")] })),
            row(3, "event", json!({ "type": "dialogueEnded", "gameCharacterId": "c1" })),
            row(4, "event", json!({ "type": "dialogueStarted", "gameCharacterId": "c1" })),
            row(5, "message", json!({ "conversationId": "conv", "role": "assistant", "lines": [line("이번")] })),
            row(6, "message", json!({ "conversationId": "other", "role": "assistant", "lines": [line("남의 대화")] })),
        ];
        let scope = CurrentScope::Talk { character_id: "c1".into(), conversation_id: Some("conv".into()) };
        let got: Vec<u32> = current_entries(&rows, &scope).iter().map(|e| e.seq).collect();
        assert_eq!(got, vec![5]);
    }

    #[test]
    fn turn_points_summarize_each_turn() {
        let rows = vec![
            row(1, "event", json!({ "type": "sceneScript", "sceneId": "s1", "lines": [line("어서 와")] })),
            row(2, "state", json!({ "current_scene_id": "s1", "location_id": "l1" })),
            row(3, "command", json!({ "type": "moveTo", "locationId": "l2" })),
            row(4, "state", json!({ "current_scene_id": null, "location_id": "l2" })),
            row(5, "message", json!({ "role": "assistant", "lines": [line("반가워요")] })),
            row(6, "state", json!({ "location_id": "l2" })),
        ];
        let points = turn_points_from(&rows);
        let got: Vec<(u32, &str)> = points.iter().map(|p| (p.seq, p.summary.as_str())).collect();
        assert_eq!(got, vec![(6, "반가워요"), (4, "이동"), (2, "어서 와")]);
        assert_eq!(points[2].scene_id.as_deref(), Some("s1"));
    }
}
