//! 정규식 치환.
//!
//! SillyTavern regex 확장과 같은 규칙으로 동작한다 (가져온 스크립트가 그대로 동작하도록).
//! - find_regex: `/pattern/flags` (JS 표기). `g`가 없으면 첫 번째 일치만 바꾼다.
//!   lookaround·역참조를 쓰는 JS 패턴이 많아 `fancy-regex`를 쓴다.
//! - replace_string: `{{match}}` = 전체 일치, `$1`… = 캡처 그룹(trim_strings를 지운 뒤 삽입),
//!   치환 결과의 `{{char}}`/`{{user}}`는 이름으로 바뀐다.
//! - 적용 순서: 전역 → 프리셋 → 캐릭터, 같은 범위 안에서는 sort_order 순.
//!
//! ## 적용 지점
//! | 시점                         | placement   | target          |
//! |-----------------------------|-------------|-----------------|
//! | 사용자 메시지 저장 (append)     | user_input  | stored          |
//! | 프롬프트 조립 (이번 입력)        | user_input  | stored + prompt |
//! | 프롬프트 조립 (대화 기록)        | 역할별       | prompt          |
//! | 프롬프트 조립 (월드인포)         | world_info  | stored + prompt |
//! | AI 응답 완료                   | ai_output   | stored          |
//! | 화면 표시 (프론트가 apply_regex) | 역할별       | display         |
use fancy_regex::{Captures, Regex, RegexBuilder};

use crate::{
    db::{self, conn::DbPool},
    error::{AppError, ErrorCode, Result},
    models::regex::{MacroMode, RegexPlacement, RegexScope, RegexScript, RegexTarget},
};

/// 정규식 코어 플러그인 스위치. 꺼져 있으면 어떤 스크립트도 불러오지 않는다 ({{char}}·{{user}} 치환은 그대로).
/// 대화·화면·저장 경로가 모두 이 모듈로 정규식을 불러오므로 한 곳에서 막는다 (sys::plugins, 설정 바꿀 때 갱신).
static ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, std::sync::atomic::Ordering::Relaxed);
}

fn enabled() -> bool {
    ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

/// 치환에 쓰는 이름
#[derive(Debug, Clone, Default)]
pub struct RegexMacros {
    pub char_name: String,
    pub user_name: String,
}

struct Compiled {
    id: String,
    regex: Regex,
    global: bool,
    /// `{{match}}`가 `$0`으로 바뀐 치환 문자열
    replace: String,
    trim_strings: Vec<String>,
    placement: Vec<RegexPlacement>,
    target: RegexTarget,
    min_depth: Option<i64>,
    max_depth: Option<i64>,
}

/// 한 문맥(캐릭터·프리셋·이름)에 대해 컴파일된 스크립트 묶음
pub struct RegexProcessor {
    scripts: Vec<Compiled>,
    macros: RegexMacros,
}

impl RegexProcessor {
    pub fn empty() -> Self {
        Self {
            scripts: vec![],
            macros: RegexMacros::default(),
        }
    }

    /// DB에서 이 문맥에 해당하는 스크립트를 불러와 컴파일
    pub async fn load(db: &DbPool, character_id: Option<&str>, preset_id: Option<&str>, macros: RegexMacros) -> Result<Self> {
        let character_id = character_id.filter(|s| !s.is_empty());
        let preset_id = preset_id.filter(|s| !s.is_empty());
        let scripts = if enabled() {
            db::regex::get_regex_scripts_for(db, character_id, preset_id).await?
        } else {
            vec![]
        };
        Ok(Self::new(scripts, macros))
    }

    /// 대화 기준으로 로드: 대화의 캐릭터(범위·{{char}})와 페르소나({{user}})를 쓴다.
    /// 대화에 캐릭터/페르소나가 없거나 지워졌으면 전역 스크립트와 기본 이름만 쓴다.
    pub async fn for_conversation(db: &DbPool, conversation_id: &str, preset_id: Option<&str>) -> Result<Self> {
        use crate::db::conversation::{get_character, get_conversation, get_persona};
        let conv = match get_conversation(db, conversation_id).await {
            Ok(c) => Some(c),
            Err(sqlx::Error::RowNotFound) => None,
            Err(e) => return Err(e.into()),
        };
        let conversation_found = conv.is_some();
        let (character_id, persona_id) = conv.map(|c| (c.character_id, c.persona_id)).unwrap_or_default();

        let char_name = match character_id.as_str() {
            "" => None,
            id => get_character(db, id).await.ok().map(|c| c.name),
        };
        // 대화가 없으면(id 없음) 이름도 없음. 있으면 대화 → 캐릭터 기본 → 전역 (ADR-028)
        let user_name = match conversation_found {
            false => None,
            true => get_persona(db, &crate::db::conversation::resolve_conv_persona(db, &persona_id, &character_id).await)
                .await
                .ok()
                .map(|p| p.name),
        };
        let macros = RegexMacros {
            char_name: char_name.unwrap_or_default(),
            user_name: user_name.unwrap_or_default(),
        };
        Self::load(db, Some(&character_id), preset_id, macros).await
    }

    /// 꺼진 스크립트는 빼고, 컴파일에 실패한 스크립트는 건너뛴다 (나머지는 계속 동작)
    pub fn new(mut scripts: Vec<RegexScript>, macros: RegexMacros) -> Self {
        scripts.retain(|s| s.enabled);
        scripts.sort_by_key(|s| (scope_rank(s.scope), s.sort_order));
        let compiled = scripts
            .iter()
            .filter_map(|s| match compile(s, &macros) {
                Ok(c) => Some(c),
                Err(e) => {
                    eprintln!("[regex] '{}' 스크립트를 건너뜀: {e}", s.name);
                    None
                }
            })
            .collect();
        Self { scripts: compiled, macros }
    }

    /// 화면에 보일 글: 표시용(Display) 정규식을 거친 뒤 {{char}}·{{user}} (옛 <BOT>·<USER>)를 이름으로
    /// `depth`: 0 = 가장 최근 메시지
    pub fn for_display(&self, text: &str, role: &str, depth: usize) -> String {
        let out = match RegexPlacement::for_role(role) {
            Some(p) => self.apply(text, p, &[RegexTarget::Display], Some(depth)),
            None => text.to_string(),
        };
        display_macros(&out, &self.macros)
    }

    pub fn is_empty(&self) -> bool {
        self.scripts.is_empty()
    }

    /// `placement`에 해당하고 `targets` 중 하나에 속하는 스크립트를 차례로 적용.
    /// `depth`: 0 = 가장 최근 메시지. None이면 깊이 제한을 보지 않는다.
    pub fn apply(&self, text: &str, placement: RegexPlacement, targets: &[RegexTarget], depth: Option<usize>) -> String {
        let mut out = text.to_string();
        for c in &self.scripts {
            if !c.placement.contains(&placement) || !targets.contains(&c.target) {
                continue;
            }
            if let Some(d) = depth.map(|d| d as i64) {
                if c.min_depth.is_some_and(|m| d < m) || c.max_depth.is_some_and(|m| d > m) {
                    continue;
                }
            }
            out = apply_one(c, &out, &self.macros);
        }
        out
    }
}

/// 스크립트 하나를 검증 (저장 전 패턴 확인용)
pub fn validate_script(script: &RegexScript) -> Result<()> {
    compile(script, &RegexMacros::default()).map(|_| ())
}

/// 저장하지 않고 스크립트 하나를 텍스트에 적용해 본다 (편집 화면 미리보기)
pub fn preview_script(script: &RegexScript, text: &str, macros: &RegexMacros) -> Result<String> {
    let c = compile(script, macros)?;
    Ok(apply_one(&c, text, macros))
}

fn scope_rank(scope: RegexScope) -> u8 {
    match scope {
        RegexScope::Global => 0,
        RegexScope::Preset => 1,
        RegexScope::Character => 2,
    }
}

// =======================================
// 컴파일
// =======================================

/// JS 표기 정규식 해석: "/pattern/flags" → (pattern, flags)
/// 슬래시가 없거나 플래그가 이상하면 전체를 플래그 없는 패턴으로 본다 (ST regexFromString과 같음)
fn parse_find_regex(input: &str) -> (&str, &str) {
    if let Some(rest) = input.strip_prefix('/') {
        if let Some(end) = rest.rfind('/') {
            let (pattern, flags) = (&rest[..end], &rest[end + 1..]);
            let valid_flags = flags.chars().all(|c| "gimsuyd".contains(c));
            if !pattern.is_empty() && valid_flags {
                return (pattern, flags);
            }
        }
    }
    (input, "")
}

fn compile(script: &RegexScript, macros: &RegexMacros) -> Result<Compiled> {
    let (pattern, flags) = parse_find_regex(&script.find_regex);
    let pattern = match script.substitute_macros {
        MacroMode::None => pattern.to_string(),
        MacroMode::Raw => substitute_macros(pattern, macros, |v| v.to_string()),
        MacroMode::Escaped => substitute_macros(pattern, macros, escape_regex),
    };
    if pattern.is_empty() {
        return Err(invalid(&script.name, "찾을 패턴이 비어 있습니다"));
    }
    let regex = RegexBuilder::new(&pattern)
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'))
        .build()
        .map_err(|e| invalid(&script.name, &e.to_string()))?;

    Ok(Compiled {
        id: script.id.clone(),
        regex,
        global: flags.contains('g'),
        replace: replace_ignore_case(&script.replace_string, "{{match}}", "$0"),
        trim_strings: script
            .trim_strings
            .iter()
            .map(|t| substitute_macros(t, macros, |v| v.to_string()))
            .filter(|t| !t.is_empty())
            .collect(),
        placement: script.placement.clone(),
        target: script.target,
        min_depth: script.min_depth,
        max_depth: script.max_depth,
    })
}

fn invalid(name: &str, reason: &str) -> AppError {
    AppError::Validation {
        code: ErrorCode::InvalidInput,
        message: format!("정규식 '{name}'이(가) 올바르지 않습니다: {reason}"),
        details: None,
    }
}

// =======================================
// 적용
// =======================================

fn apply_one(c: &Compiled, text: &str, macros: &RegexMacros) -> String {
    let limit = if c.global { 0 } else { 1 };
    let replacer = |caps: &Captures<'_, str>| {
        let expanded = expand_groups(&c.replace, caps, &c.trim_strings);
        substitute_macros(&expanded, macros, |v| v.to_string())
    };
    match c.regex.try_replacen(text, limit, replacer) {
        Ok(result) => result.into_owned(),
        Err(e) => {
            // 백트래킹 한도 초과 등 실행 오류: 이 스크립트만 건너뜀
            eprintln!("[regex] {} 실행 실패: {e}", c.id);
            text.to_string()
        }
    }
}

/// `$N`을 캡처 그룹으로 바꾼다. 없는 그룹은 빈 문자열. 삽입 전에 trim_strings를 지운다.
fn expand_groups(template: &str, caps: &Captures<'_, str>, trim_strings: &[String]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        if ch == '$' {
            let digits: String = template[i + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                for _ in 0..digits.len() {
                    chars.next();
                }
                let n: usize = digits.parse().unwrap_or(usize::MAX);
                if let Some(m) = caps.get(n) {
                    let mut group = m.as_str().to_string();
                    for t in trim_strings {
                        group = group.replace(t.as_str(), "");
                    }
                    out.push_str(&group);
                }
                continue;
            }
        }
        out.push(ch);
    }
    out
}

/// `{{char}}` / `{{user}}` 치환 (대소문자 무시). 모르는 매크로는 그대로 둔다.
fn substitute_macros(text: &str, macros: &RegexMacros, encode: impl Fn(&str) -> String) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return out;
        };
        match after[..end].trim().to_ascii_lowercase().as_str() {
            "char" => out.push_str(&encode(&macros.char_name)),
            "user" => out.push_str(&encode(&macros.user_name)),
            _ => out.push_str(&rest[start..start + end + 4]),
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

fn escape_regex(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if "\\.+*?()|[]{}^$#&-~".contains(ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn replace_ignore_case(text: &str, needle: &str, with: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (i, _) in lower.match_indices(&needle) {
        out.push_str(&text[last..i]);
        out.push_str(with);
        last = i + needle.len();
    }
    out.push_str(&text[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(find: &str, replace: &str) -> RegexScript {
        RegexScript {
            id: format!("s-{find}"),
            name: find.into(),
            find_regex: find.into(),
            replace_string: replace.into(),
            trim_strings: vec![],
            placement: vec![RegexPlacement::AiOutput],
            target: RegexTarget::Stored,
            scope: RegexScope::Global,
            scope_id: None,
            substitute_macros: MacroMode::None,
            run_on_edit: true,
            min_depth: None,
            max_depth: None,
            enabled: true,
            sort_order: 0,
            source_format: "native".into(),
            extra_json: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn macros() -> RegexMacros {
        RegexMacros {
            char_name: "Aria".into(),
            user_name: "Min.".into(),
        }
    }

    fn run(scripts: Vec<RegexScript>, text: &str) -> String {
        RegexProcessor::new(scripts, macros()).apply(text, RegexPlacement::AiOutput, &[RegexTarget::Stored], Some(0))
    }

    #[test]
    fn global_flag_controls_replace_count() {
        assert_eq!(run(vec![script("/a/", "b")], "aaa"), "baa");
        assert_eq!(run(vec![script("/a/g", "b")], "aaa"), "bbb");
        // 슬래시 없는 패턴 = 플래그 없음
        assert_eq!(run(vec![script("a", "b")], "aaa"), "baa");
        assert_eq!(run(vec![script("/A/gi", "b")], "aAa"), "bbb");
    }

    #[test]
    fn groups_match_and_trim_strings() {
        let mut s = script(r"/\*(.+?)\*/g", "<i>$1</i> [{{match}}] $9");
        s.trim_strings = vec!["!".into()];
        // trim_strings는 {{match}}($0)에도 적용됨 (ST와 같음), 없는 그룹($9)은 빈 문자열
        assert_eq!(run(vec![s], "*wow!* ok"), "<i>wow</i> [*wow*]  ok");
    }

    #[test]
    fn macros_in_replacement_and_find() {
        assert_eq!(run(vec![script("/me/g", "{{char}}")], "me and me"), "Aria and Aria");

        let mut raw = script("/{{user}}/g", "U");
        raw.substitute_macros = MacroMode::Raw;
        // "Min." 의 '.'이 정규식으로 해석됨 → "Mine"도 일치
        assert_eq!(run(vec![raw], "Min. Mine"), "U U");

        let mut esc = script("/{{user}}/g", "U");
        esc.substitute_macros = MacroMode::Escaped;
        assert_eq!(run(vec![esc], "Min. Mine"), "U Mine");
    }

    #[test]
    fn js_lookaround_is_supported() {
        assert_eq!(run(vec![script(r"/(?<=\d)px/g", "")], "10px apx"), "10 apx");
        assert_eq!(run(vec![script(r#"/"(?=\w)/g"#, "“")], r#""hi" "#), r#"“hi" "#);
    }

    #[test]
    fn filters_by_placement_target_and_depth() {
        let mut user_only = script("/x/g", "U");
        user_only.placement = vec![RegexPlacement::UserInput];
        let mut prompt_only = script("/y/g", "P");
        prompt_only.target = RegexTarget::Prompt;
        let mut deep = script("/z/g", "D");
        deep.min_depth = Some(2);
        let p = RegexProcessor::new(vec![user_only, prompt_only, deep], macros());

        assert_eq!(p.apply("xyz", RegexPlacement::AiOutput, &[RegexTarget::Stored], Some(0)), "xyz");
        assert_eq!(
            p.apply("xyz", RegexPlacement::AiOutput, &[RegexTarget::Stored, RegexTarget::Prompt], Some(3)),
            "xPD"
        );
        assert_eq!(p.apply("xyz", RegexPlacement::UserInput, &[RegexTarget::Stored], None), "Uyz");
    }

    #[test]
    fn order_is_global_preset_character_then_sort_order() {
        let mut ch = script("/a/", "c");
        ch.scope = RegexScope::Character;
        let mut pre = script("/a/", "p");
        pre.scope = RegexScope::Preset;
        let mut g2 = script("/^/", "2");
        g2.sort_order = 2;
        let mut g1 = script("/^/", "1");
        g1.sort_order = 1;
        // 전역(1, 2) → 프리셋(a→p) → 캐릭터(다음 a→c)
        assert_eq!(run(vec![ch, pre, g2, g1], "aa"), "21pc");
    }

    #[test]
    fn disabled_and_invalid_scripts_are_skipped() {
        let mut off = script("/a/g", "X");
        off.enabled = false;
        let broken = script("/(unclosed/g", "X");
        assert!(validate_script(&broken).is_err());
        assert_eq!(run(vec![off, broken, script("/b/g", "B")], "ab"), "aB");
    }

    #[tokio::test]
    async fn loads_scoped_scripts_from_db() {
        use crate::models::regex::RegexScriptInput;
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();

        let input = |name: &str, find: &str, rep: &str, scope: RegexScope, scope_id: Option<&str>| RegexScriptInput {
            id: None,
            name: name.into(),
            find_regex: find.into(),
            replace_string: rep.into(),
            trim_strings: vec!["!".into()],
            placement: vec![RegexPlacement::AiOutput, RegexPlacement::UserInput],
            target: RegexTarget::Stored,
            scope,
            scope_id: scope_id.map(str::to_string),
            substitute_macros: MacroMode::None,
            run_on_edit: true,
            min_depth: None,
            max_depth: Some(3),
            enabled: true,
            sort_order: 0,
            source_format: None,
            extra_json: None,
        };
        for i in [
            input("g", "/a/g", "G", RegexScope::Global, None),
            input("mine", "/b/g", "{{char}}", RegexScope::Character, Some("c1")),
            input("other", "/c/g", "X", RegexScope::Character, Some("c2")),
            input("pre", "/d/g", "P", RegexScope::Preset, Some("p1")),
        ] {
            db::regex::upsert_regex_script(&pool, &i).await.unwrap();
        }

        let all = db::regex::get_all_regex_scripts(&pool).await.unwrap();
        assert_eq!(all.len(), 4);
        let g = all.iter().find(|s| s.name == "g").unwrap();
        assert_eq!(g.placement, vec![RegexPlacement::AiOutput, RegexPlacement::UserInput]);
        assert_eq!(g.trim_strings, vec!["!".to_string()]);
        assert_eq!(g.max_depth, Some(3));

        let p = RegexProcessor::load(&pool, Some("c1"), Some("p1"), macros()).await.unwrap();
        assert_eq!(p.apply("abcd", RegexPlacement::AiOutput, &[RegexTarget::Stored], Some(0)), "GAriacP");
        // 범위 밖 캐릭터/프리셋 없음 → 전역만
        let p = RegexProcessor::load(&pool, None, None, macros()).await.unwrap();
        assert_eq!(p.apply("abcd", RegexPlacement::UserInput, &[RegexTarget::Stored], None), "Gbcd");
        // max_depth 3을 넘으면 적용 안 됨
        assert_eq!(p.apply("a", RegexPlacement::UserInput, &[RegexTarget::Stored], Some(4)), "a");
    }

    #[test]
    fn preview_ignores_placement_and_target() {
        let mut s = script("/hi/g", "hello");
        s.target = RegexTarget::Display;
        assert_eq!(preview_script(&s, "hi hi", &macros()).unwrap(), "hello hello");
    }
}

/// 표시용 이름 바꾸기 (대소문자 무시)
pub fn display_macros(text: &str, m: &RegexMacros) -> String {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"(?i)\{\{(char|user)\}\}|<(bot|user)>").unwrap());
    RE.replace_all(text, |c: &Captures<'_, str>| {
        let who = c.get(1).or(c.get(2)).map(|x| x.as_str().to_ascii_lowercase()).unwrap_or_default();
        let name = if who == "user" { &m.user_name } else { &m.char_name };
        if name.is_empty() {
            c[0].to_string()
        } else {
            name.clone()
        }
    })
    .into_owned()
}

#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    fn names_fill_in_on_display() {
        let m = RegexMacros {
            char_name: "Aria".into(),
            user_name: "Min".into(),
        };
        assert_eq!(display_macros("{{User}}, <BOT> waves. {{char}}!", &m), "Min, Aria waves. Aria!");
        let none = RegexMacros::default();
        assert_eq!(display_macros("{{user}}", &none), "{{user}}", "이름 없으면 그대로");
    }
}
