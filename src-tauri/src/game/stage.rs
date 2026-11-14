//! 무대 연출: 텍스트(제작자의 씬 대본, AI의 NPC 응답)를 "한 줄 + 연출" 목록(StageLine)으로 바꾼다.
//! 제작자가 쓰는 문법과 AI가 쓰는 문법은 **같다**. 다른 건 허용 범위(Allowed)뿐이다.
//!
//! 한 줄 = 한 노드(줄바꿈 기준). 줄 앞에 `이름:`이 있고 그 이름이 캐릭터이면 대사, 아니면 나레이션.
//! 연출 태그는 줄의 어디에 있든 그 줄을 보여주기 "직전"에 실행된다. 태그만 있는 줄은 다음 줄에 합쳐진다.
//!
//!   <bgm>peaceful</bgm> / <bgm>stop</bgm>   배경음 분위기 (BGM_MOODS 중 하나)
//!   <sfx>새 소리</sfx>                       효과음 그룹 이름 (같은 이름의 변형 중 하나가 랜덤으로 뽑힌다)
//!   <cg>festival_01</cg> / <cg>off</cg>      이벤트 CG (위치와 무관하게 화면에 겹쳐진다)
//!   <voice>yuna_greet</voice>                미리 녹음된 음성 (제작자 대본 전용 — AI는 못 쓴다)
//!   <effects>shake</effects>                 화면 효과 (SCREEN_EFFECTS), 선택적으로 `이름:값` (예: blackout:2s)
//!   <emotion::유나>happy</emotion>           그 캐릭터의 스탠딩을 감정에 맞는 이미지로 바꾼다 (같은 줄의 화자가 아니어도 된다)
//!   <motion::유나>jump</motion>              그 캐릭터가 움직인다 (MOTIONS)
//! 배경은 태그가 아니라 위치/시간대/날씨에서 엔진이 자동으로 고른다.
//!
//! 키를 파일로 푸는 일은 StageResolver가 한다(GameDefinition 쪽 구현). 풀 수 없는 키, 허용되지 않은 태그,
//! 상한을 넘는 태그는 조용히 버린다 — AI 출력은 신뢰하지 않는 입력이다.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 배경음 분위기 (영문 id). 곡은 분위기별로 여러 개일 수 있고, 없으면 default로 대체한다. 표시 이름은 프론트가 정한다.
pub const BGM_MOODS: [&str; 6] = ["default", "peaceful", "upbeat", "romance", "tense", "battle"];
/// 기본 제공 화면 효과. 실제 그리기는 프론트(pixi.js)가 한다. clear는 지속형 효과(petals/snow/rain/fog)를 끈다.
pub const SCREEN_EFFECTS: [&str; 10] = ["shake", "flash", "blackout", "fade_in", "fireworks", "petals", "snow", "rain", "fog", "clear"];
/// 기본 제공 캐릭터 모션 (스탠딩 스프라이트에 적용하는 프리셋).
pub const MOTIONS: [&str; 8] = ["jump", "bounce", "shake", "nod", "sway", "shrink", "slide_in", "slide_out"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetRef {
    pub id: String,
    pub key: String,
    /// games/<game_id>/ 기준 상대경로. iframe이 그대로 쓴다.
    pub path: String,
    /// 볼륨, 루프, 페이드 같은 재생 설정 (game_assets.settings_json)
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Cue {
    /// asset이 None이면 정지
    Bgm { mood: String, asset: Option<AssetRef> },
    Sfx { tag: String, asset: AssetRef },
    Voice { asset: AssetRef },
    Cg { asset: AssetRef },
    CgOff,
    Standing { character_id: String, emotion: String, asset: AssetRef },
    StandingOff { character_id: String },
    Motion { character_id: String, name: String },
    Effect { name: String, param: Option<String> },
    /// 텍스트 태그가 아니라 엔진이 위치/시간/날씨가 바뀔 때 자동으로 낸다.
    Background { asset: AssetRef },
    /// 지금 위치/시간/날씨에 맞는 배경이 하나도 없을 때 이전 배경을 내린다.
    BackgroundOff,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LineKind {
    Narration,
    Dialogue,
    /// 텍스트 없이 연출만 있는 줄
    Direction,
}

/// 프론트로 나가는 한 줄. cues는 이 줄을 보여주기 직전에 순서대로 실행한다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StageLine {
    pub kind: LineKind,
    /// game_character_id. 플레이어는 "player", 나레이션/Direction은 None.
    pub speaker: Option<String>,
    pub text: String,
    pub cues: Vec<Cue>,
}

/// 키를 구체적인 파일로 푸는 쪽 (GameDefinition + RNG). 못 풀면 None.
pub trait StageResolver {
    /// 이름 또는 id → game_character_id
    fn character_by_name(&self, name: &str) -> Option<String>;
    fn is_player_name(&self, name: &str) -> bool;
    fn bgm(&mut self, mood: &str) -> Option<AssetRef>;
    fn sfx(&mut self, tag: &str) -> Option<AssetRef>;
    fn cg(&mut self, key: &str) -> Option<AssetRef>;
    fn voice(&mut self, key: &str) -> Option<AssetRef>;
    fn standing(&mut self, character_id: &str, emotion: &str) -> Option<AssetRef>;
}

/// 어떤 연출을 허용하는가. 제작자 대본과 AI 응답은 문법이 같고 이것만 다르다.
#[derive(Debug, Clone)]
pub struct Allowed {
    pub bgm: bool,
    pub sfx: bool,
    pub cg: bool,
    pub voice: bool,
    pub effects: bool,
    pub emotion: bool,
    pub motion: bool,
    /// 플레이어 대사(`나:`)를 허용하는가. AI는 플레이어 대신 말할 수 없다.
    pub player_lines: bool,
    /// Some이면 그 캐릭터들만 화자/연출 대상이 될 수 있다 (AI: 대화 상대 + 같은 위치에 있는 NPC).
    pub speakers: Option<HashSet<String>>,
    pub max_cues: usize,
    pub max_bgm: usize,
    pub max_cg: usize,
}

impl Allowed {
    pub fn author() -> Self {
        Self { bgm: true, sfx: true, cg: true, voice: true, effects: true, emotion: true, motion: true, player_lines: true, speakers: None, max_cues: usize::MAX, max_bgm: usize::MAX, max_cg: usize::MAX }
    }

    /// AI 응답: 음성은 못 쓰고(미래의 음성 생성 연결을 위해 비워둔다), 플레이어 대사도 못 쓰며, 화자는 정해진 캐릭터만.
    pub fn ai(speakers: HashSet<String>) -> Self {
        Self { bgm: true, sfx: true, cg: true, voice: false, effects: true, emotion: true, motion: true, player_lines: false, speakers: Some(speakers), max_cues: 6, max_bgm: 1, max_cg: 1 }
    }

    fn speaker_ok(&self, id: &str) -> bool {
        self.speakers.as_ref().map_or(true, |s| s.contains(id))
    }
}

struct RawCue {
    name: String,
    target: Option<String>,
    value: String,
}

const TAGS: [&str; 7] = ["bgm", "sfx", "cg", "voice", "effects", "emotion", "motion"];

/// 한 줄에서 연출 태그를 뽑고 나머지 텍스트를 돌려준다. 우리 태그가 아닌 `<...>`는 텍스트로 둔다.
/// 닫는 태그는 `</emotion>`, `</emotion::유나>` 어느 쪽이든 받는다 (LLM이 닫는 태그에도 대상을 붙이는 경우가 있다).
fn extract_cues(line: &str) -> (String, Vec<RawCue>) {
    let mut text = String::new();
    let mut cues = Vec::new();
    let mut rest = line;
    loop {
        let Some(start) = rest.find('<') else {
            text.push_str(rest);
            break;
        };
        text.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(gt) = after.find('>') else {
            text.push_str(&rest[start..]);
            break;
        };
        let head = &after[..gt];
        let (name, target) = match head.split_once("::") {
            Some((n, t)) => (n.trim(), Some(t.trim().to_string())),
            None => (head.trim(), None),
        };
        if !TAGS.contains(&name) {
            text.push('<');
            rest = after;
            continue;
        }
        let body = &after[gt + 1..];
        let close = format!("</{name}");
        match body.find(&close) {
            Some(cpos) => {
                let after_close = &body[cpos + close.len()..];
                match after_close.find('>') {
                    Some(cgt) => {
                        cues.push(RawCue { name: name.to_string(), target, value: body[..cpos].trim().to_string() });
                        rest = &after_close[cgt + 1..];
                    }
                    None => {
                        text.push_str(&rest[start..start + gt + 2]);
                        rest = body;
                    }
                }
            }
            None => {
                // 닫는 태그가 없으면 연출이 아니라 텍스트로 둔다 (잘린 응답 등)
                text.push_str(&rest[start..start + gt + 2]);
                rest = body;
            }
        }
    }
    (text, cues)
}

/// "이름: 본문" → (이름, 본문). 콜론은 반각/전각 모두. 이름이 너무 길거나 기호가 섞여 있으면 화자 표기가 아니다.
fn split_speaker(text: &str) -> Option<(&str, &str)> {
    let t = text.trim();
    let (idx, ch) = t.char_indices().find(|(_, c)| *c == ':' || *c == '：')?;
    let name = t[..idx].trim();
    let body = t[idx + ch.len_utf8()..].trim();
    if name.is_empty() || name.chars().count() > 20 || name.contains(['<', '>', '"', '(', ')', '*', '[', ']']) {
        return None;
    }
    Some((name, body))
}

struct Budget {
    total: usize,
    bgm: usize,
    cg: usize,
}

fn clean_param(p: &str) -> Option<String> {
    let p = p.trim();
    if p.is_empty() || p.chars().count() > 16 || !p.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-') {
        None
    } else {
        Some(p.to_string())
    }
}

fn resolve_cue(rc: &RawCue, r: &mut dyn StageResolver, allow: &Allowed, b: &mut Budget) -> Option<Cue> {
    if b.total >= allow.max_cues {
        return None;
    }
    let v = rc.value.as_str();
    let cue = match rc.name.as_str() {
        "bgm" if allow.bgm && b.bgm < allow.max_bgm => {
            if v.eq_ignore_ascii_case("stop") {
                Cue::Bgm { mood: "stop".into(), asset: None }
            } else {
                let mood = v.to_ascii_lowercase();
                if !BGM_MOODS.contains(&mood.as_str()) {
                    return None;
                }
                let asset = r.bgm(&mood)?;
                Cue::Bgm { mood, asset: Some(asset) }
            }
        }
        "sfx" if allow.sfx => Cue::Sfx { tag: v.to_string(), asset: r.sfx(v)? },
        "cg" if allow.cg && b.cg < allow.max_cg => {
            if v.eq_ignore_ascii_case("off") {
                Cue::CgOff
            } else {
                Cue::Cg { asset: r.cg(v)? }
            }
        }
        "voice" if allow.voice => Cue::Voice { asset: r.voice(v)? },
        "effects" if allow.effects => {
            let (name, param) = match v.split_once(':') {
                Some((n, p)) => (n.trim(), clean_param(p)),
                None => (v, None),
            };
            let name = name.to_ascii_lowercase();
            if !SCREEN_EFFECTS.contains(&name.as_str()) {
                return None;
            }
            Cue::Effect { name, param }
        }
        "emotion" if allow.emotion => {
            let id = r.character_by_name(rc.target.as_deref()?)?;
            if !allow.speaker_ok(&id) {
                return None;
            }
            if v.eq_ignore_ascii_case("off") {
                Cue::StandingOff { character_id: id }
            } else {
                let asset = r.standing(&id, v)?;
                Cue::Standing { character_id: id, emotion: v.to_string(), asset }
            }
        }
        "motion" if allow.motion => {
            let id = r.character_by_name(rc.target.as_deref()?)?;
            let name = v.to_ascii_lowercase();
            if !allow.speaker_ok(&id) || !MOTIONS.contains(&name.as_str()) {
                return None;
            }
            Cue::Motion { character_id: id, name }
        }
        _ => return None,
    };
    b.total += 1;
    match &cue {
        Cue::Bgm { .. } => b.bgm += 1,
        Cue::Cg { .. } => b.cg += 1,
        _ => {}
    }
    Some(cue)
}

/// 텍스트를 줄 목록으로 파싱한다. 빈 줄은 건너뛴다.
pub fn parse(raw: &str, r: &mut dyn StageResolver, allow: &Allowed) -> Vec<StageLine> {
    let mut lines = Vec::new();
    let mut pending: Vec<Cue> = Vec::new();
    let mut budget = Budget { total: 0, bgm: 0, cg: 0 };

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (text, raw_cues) = extract_cues(line);
        let cues: Vec<Cue> = raw_cues.iter().filter_map(|rc| resolve_cue(rc, r, allow, &mut budget)).collect();
        let text = text.trim().to_string();
        if text.is_empty() {
            pending.extend(cues); // 태그만 있는 줄은 다음 줄에 합친다
            continue;
        }

        let (kind, speaker, body) = match split_speaker(&text) {
            Some((name, body)) => {
                if let Some(id) = r.character_by_name(name) {
                    if !allow.speaker_ok(&id) {
                        continue; // 허용되지 않은 화자(이 자리에 없는 인물 등)의 대사는 버린다
                    }
                    (LineKind::Dialogue, Some(id), body.to_string())
                } else if r.is_player_name(name) {
                    if !allow.player_lines {
                        continue; // AI는 플레이어 대신 말하지 못한다
                    }
                    (LineKind::Dialogue, Some("player".to_string()), body.to_string())
                } else {
                    (LineKind::Narration, None, text.clone()) // 모르는 이름의 접두는 그냥 나레이션 텍스트
                }
            }
            None => (LineKind::Narration, None, text.clone()),
        };
        if body.is_empty() {
            pending.extend(cues);
            continue;
        }
        let mut all = std::mem::take(&mut pending);
        all.extend(cues);
        lines.push(StageLine { kind, speaker, text: body, cues: all });
    }
    if !pending.is_empty() {
        lines.push(StageLine { kind: LineKind::Direction, speaker: None, text: String::new(), cues: pending });
    }
    lines
}

/// 큐를 무대 상태(stage_json)에 반영한다: {background, bgm:{mood,asset}, cg, standings:{캐릭터id:{emotion,asset}}}.
/// 효과음/음성/화면 효과/모션은 한 번 일어나고 끝나는 것이라 상태에 남기지 않는다.
pub fn apply_cue_to_stage(stage: &mut serde_json::Value, cue: &Cue) {
    use serde_json::json;
    if !stage.is_object() {
        *stage = json!({});
    }
    let Some(obj) = stage.as_object_mut() else { return };
    match cue {
        Cue::Bgm { mood, asset } => {
            if mood == "stop" || asset.is_none() {
                obj.remove("bgm");
            } else {
                obj.insert("bgm".into(), json!({ "mood": mood, "asset": asset }));
            }
        }
        Cue::Cg { asset } => {
            obj.insert("cg".into(), json!(asset));
        }
        Cue::CgOff => {
            obj.remove("cg");
        }
        Cue::Background { asset } => {
            obj.insert("background".into(), json!(asset));
        }
        Cue::BackgroundOff => {
            obj.remove("background");
        }
        Cue::Standing { character_id, emotion, asset } => {
            let standings = obj.entry("standings").or_insert_with(|| json!({}));
            if let Some(m) = standings.as_object_mut() {
                m.insert(character_id.clone(), json!({ "emotion": emotion, "asset": asset }));
            }
        }
        Cue::StandingOff { character_id } => {
            if let Some(m) = obj.get_mut("standings").and_then(|s| s.as_object_mut()) {
                m.remove(character_id);
            }
        }
        Cue::Sfx { .. } | Cue::Voice { .. } | Cue::Effect { .. } | Cue::Motion { .. } => {}
    }
}

/// 이벤트 효과의 `stage` 동사를 대본 문법으로 바꾼다. 그러면 같은 파서/검증/해석을 그대로 쓴다.
///   {"stage": {"bgm":"romance", "sfx":"문 소리", "cg":"festival_01"|"off", "voice":"yuna_greet",
///              "effects":["fireworks","shake"], "emotion":{"유나":"happy"}, "motion":{"유나":"jump"}}}
pub fn spec_to_script(spec: &serde_json::Value) -> String {
    let ok = |s: &str| !s.contains(['<', '>']);
    let mut out = String::new();
    for tag in ["bgm", "sfx", "cg", "voice"] {
        if let Some(v) = spec.get(tag).and_then(|v| v.as_str()).filter(|v| ok(v)) {
            out.push_str(&format!("<{tag}>{v}</{tag}>"));
        }
    }
    match spec.get("effects") {
        Some(serde_json::Value::Array(arr)) => {
            for e in arr.iter().filter_map(|x| x.as_str()).filter(|e| ok(e)) {
                out.push_str(&format!("<effects>{e}</effects>"));
            }
        }
        Some(serde_json::Value::String(e)) if ok(e) => out.push_str(&format!("<effects>{e}</effects>")),
        _ => {}
    }
    for tag in ["emotion", "motion"] {
        if let Some(m) = spec.get(tag).and_then(|v| v.as_object()) {
            for (who, v) in m {
                if let Some(v) = v.as_str().filter(|v| ok(v) && ok(who)) {
                    out.push_str(&format!("<{tag}::{who}>{v}</{tag}>"));
                }
            }
        }
    }
    out
}

/// 대본 안의 연출 태그를 해석하지 않고 그대로 뽑는다: (태그 이름, `::대상`, 값). 에셋을 지우기 전에 어디서 쓰는지 찾는 데 쓴다.
pub fn tag_uses(raw: &str) -> Vec<(String, Option<String>, String)> {
    raw.lines().flat_map(|l| extract_cues(l.trim()).1).map(|rc| (rc.name, rc.target, rc.value)).collect()
}

/// 제작자 대본을 미리 점검한다: 쓸 수 없어서 버려질 태그를 알려준다 (키가 없음, 알 수 없는 감정/효과/분위기 등).
pub fn lint(raw: &str, r: &mut dyn StageResolver, allow: &Allowed) -> Vec<String> {
    let mut problems = Vec::new();
    for line in raw.lines() {
        let (_, raw_cues) = extract_cues(line.trim());
        for rc in raw_cues {
            let mut budget = Budget { total: 0, bgm: 0, cg: 0 };
            if resolve_cue(&rc, r, allow, &mut budget).is_none() {
                let target = rc.target.as_deref().map(|t| format!("::{t}")).unwrap_or_default();
                problems.push(format!("<{}{}>{}</{}>", rc.name, target, rc.value, rc.name));
            }
        }
    }
    problems
}

/// 저장용 평문: 태그를 벗기고 대사에는 `이름: ` 접두를 되살린다 (이전 기록이 다음 프롬프트에 들어갈 때
/// AI가 같은 형식을 계속 쓰도록). Direction 줄은 건너뛴다.
pub fn to_plain(lines: &[StageLine], name_of: &dyn Fn(&str) -> String) -> String {
    lines
        .iter()
        .filter(|l| l.kind != LineKind::Direction)
        .map(|l| match &l.speaker {
            Some(id) => format!("{}: {}", name_of(id), l.text),
            None => l.text.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Mock;
    fn a(key: &str) -> AssetRef {
        AssetRef { id: format!("id-{key}"), key: key.to_string(), path: format!("assets/{key}.x"), settings: serde_json::Value::Null }
    }
    impl StageResolver for Mock {
        fn character_by_name(&self, name: &str) -> Option<String> {
            match name {
                "유나" => Some("c1".into()),
                "미나" => Some("c2".into()),
                "철수" => Some("c3".into()),
                _ => None,
            }
        }
        fn is_player_name(&self, name: &str) -> bool {
            name == "나"
        }
        fn bgm(&mut self, mood: &str) -> Option<AssetRef> {
            Some(a(&format!("bgm_{mood}")))
        }
        fn sfx(&mut self, tag: &str) -> Option<AssetRef> {
            (tag == "문 소리").then(|| a("door"))
        }
        fn cg(&mut self, key: &str) -> Option<AssetRef> {
            (key == "festival").then(|| a("festival"))
        }
        fn voice(&mut self, key: &str) -> Option<AssetRef> {
            Some(a(key))
        }
        fn standing(&mut self, character_id: &str, emotion: &str) -> Option<AssetRef> {
            (emotion == "happy" || emotion == "surprised").then(|| a(&format!("{character_id}_{emotion}")))
        }
    }

    fn ai() -> Allowed {
        Allowed::ai(["c1".to_string(), "c2".to_string()].into_iter().collect())
    }

    #[test]
    fn splits_narration_and_dialogue_and_merges_tag_only_lines() {
        let raw = "<bgm>peaceful</bgm>\n교실에 햇빛이 든다.\n유나: 안녕! <emotion::유나>happy</emotion>\n<sfx>문 소리</sfx>\n미나: 어서 와.";
        let lines = parse(raw, &mut Mock, &Allowed::author());
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].kind, LineKind::Narration);
        assert!(matches!(lines[0].cues[0], Cue::Bgm { .. })); // 태그만 있던 줄이 다음 줄에 합쳐짐
        assert_eq!(lines[1].speaker.as_deref(), Some("c1"));
        assert_eq!(lines[1].text, "안녕!");
        assert!(matches!(lines[1].cues[0], Cue::Standing { .. }));
        assert!(matches!(lines[2].cues[0], Cue::Sfx { .. })); // sfx도 다음 줄(미나)에 붙음
    }

    #[test]
    fn emotion_can_target_someone_other_than_the_speaker() {
        let lines = parse("유나: 어, 미나?! <emotion::미나>surprised</emotion>", &mut Mock, &ai());
        match &lines[0].cues[0] {
            Cue::Standing { character_id, emotion, .. } => {
                assert_eq!(character_id, "c2");
                assert_eq!(emotion, "surprised");
            }
            c => panic!("{c:?}"),
        }
    }

    #[test]
    fn tolerant_closing_tag_and_unknown_things_are_dropped() {
        let raw = "유나: 응 <emotion::유나>happy</emotion::유나> <emotion::유나>angry</emotion> <sfx>없는소리</sfx> <effects>explode</effects>";
        let lines = parse(raw, &mut Mock, &ai());
        assert_eq!(lines[0].text, "응");
        assert_eq!(lines[0].cues.len(), 1); // happy만 남고, 없는 감정/소리/효과는 버려짐
    }

    #[test]
    fn ai_restrictions() {
        let raw = "나: 내가 말하는 척\n철수: 이 자리에 없는 사람\n유나: 정상 <voice>v1</voice>";
        let lines = parse(raw, &mut Mock, &ai());
        assert_eq!(lines.len(), 1); // 플레이어 대사와 허용되지 않은 화자의 대사는 버려진다
        assert!(lines[0].cues.is_empty()); // 음성은 AI가 못 쓴다
        let author = parse(raw, &mut Mock, &Allowed::author());
        assert_eq!(author.len(), 3);
        assert_eq!(author[0].speaker.as_deref(), Some("player"));
    }

    #[test]
    fn ai_cue_budget() {
        let raw = "유나: 하나 <bgm>peaceful</bgm><bgm>tense</bgm>\n미나: 둘 <cg>festival</cg><cg>festival</cg>";
        let lines = parse(raw, &mut Mock, &ai());
        let bgms = lines.iter().flat_map(|l| &l.cues).filter(|c| matches!(c, Cue::Bgm { .. })).count();
        let cgs = lines.iter().flat_map(|l| &l.cues).filter(|c| matches!(c, Cue::Cg { .. })).count();
        assert_eq!((bgms, cgs), (1, 1));
    }

    #[test]
    fn trailing_tags_become_a_direction_line_and_plain_text_restores_names() {
        let lines = parse("유나: 끝.\n<effects>fireworks</effects>", &mut Mock, &Allowed::author());
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].kind, LineKind::Direction);
        let plain = to_plain(&lines, &|id| if id == "c1" { "유나".into() } else { id.into() });
        assert_eq!(plain, "유나: 끝.");
    }

    #[test]
    fn stage_state_follows_cues_and_spec_becomes_a_direction_line() {
        let lines = parse(&spec_to_script(&serde_json::json!({ "bgm": "romance", "cg": "festival", "emotion": { "유나": "happy" }, "effects": ["shake"] })), &mut Mock, &Allowed::author());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].kind, LineKind::Direction);
        let mut stage = serde_json::json!({});
        for c in &lines[0].cues {
            apply_cue_to_stage(&mut stage, c);
        }
        assert_eq!(stage["bgm"]["mood"], "romance");
        assert_eq!(stage["cg"]["key"], "festival");
        assert_eq!(stage["standings"]["c1"]["emotion"], "happy");
        apply_cue_to_stage(&mut stage, &Cue::CgOff);
        apply_cue_to_stage(&mut stage, &Cue::StandingOff { character_id: "c1".into() });
        assert!(stage.get("cg").is_none() && stage["standings"].as_object().unwrap().is_empty());
    }

    #[test]
    fn unclosed_tags_stay_as_text() {
        let lines = parse("유나: 안녕 <sfx>문 소리", &mut Mock, &Allowed::author());
        assert!(lines[0].text.contains("<sfx>"));
    }
}
