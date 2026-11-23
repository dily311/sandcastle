//! 대필 (CLAUDE.md 5-D, ADR-018): 사용자가 쓴 초안(요지)을 AI가 사용자의 말투로 다음 대사로 써 준다. 결과는 입력칸에 들어갈 뿐 저장·전송되지 않는다.
//!
//! 이 파일은 지시 글 만들기와 결과 정리(순수 함수)이고, 컨텍스트를 모아 부르는 쪽은 `ChatManager::ghost`.
use crate::{models::settings::GhostSetting, service::memory::chunker};

/// 초안 최대 글자
pub const MAX_BRIEF_CHARS: usize = 1000;
/// 추가 지침 한 줄 최대 글자
pub const MAX_GUIDE_CHARS: usize = 300;
/// 결과 최대 글자 (입력칸에 들어가는 글이므로)
pub const MAX_OUTPUT_CHARS: usize = 2000;
/// 문체 예시로 보여 줄 사용자의 최근 글 수와 글자 수
const SAMPLES: usize = 3;
const SAMPLE_CHARS: usize = 300;

/// 응답 한도: 생각하는 모델은 이 예산을 추론에 쓰므로 작게 잡으면 빈 응답이 나온다. 길이는 지시 글로 정하고 한도는 넉넉히
pub fn max_tokens(length: &str) -> u32 {
    match length {
        "short" => 512,
        "long" => 1024,
        _ => 768,
    }
}

pub fn clean_brief(b: &str) -> Option<String> {
    let t: String = b.trim().chars().take(MAX_BRIEF_CHARS).collect();
    (!t.is_empty()).then_some(t)
}

/// 문체 예시: 사용자(나)가 쓴 최근 글 (태그·코드를 뺀 것)
pub fn style_samples(history: &[(String, String)]) -> Vec<String> {
    let mut out: Vec<String> = history
        .iter()
        .rev()
        .filter(|(role, _)| role == "user")
        .map(|(_, c)| chunker::clean(c).chars().take(SAMPLE_CHARS).collect::<String>())
        .filter(|c| !c.is_empty())
        .take(SAMPLES)
        .collect();
    out.reverse();
    out
}

/// 맨 끝에 붙이는 지시 (프리셋의 뒤쪽 블록이 덮어쓰지 못하게 마지막 user 메시지로)
pub fn build_instruction(me: &str, other: &str, brief: Option<&str>, samples: &[String], cfg: &GhostSetting) -> String {
    let len = match cfg.length.as_str() {
        "short" => "one or two sentences",
        "long" => "several sentences (a paragraph or more)",
        _ => "about three or four sentences",
    };
    let mut s = format!(
        "[Ghostwrite request]\nContinuing the conversation so far, write what {me} says next, from {me}'s point of view and in {me}'s voice.\n\
         - Do not write {other}'s words, actions or narration. Write only {me}'s lines (and {me}'s actions if needed).\n\
         - Length: {len}. Follow the style and format of {me}'s earlier messages, and write in the language of the conversation.\n\
         - Output only the text {me} would send, with no explanation or preface."
    );
    if !samples.is_empty() {
        s.push_str(&format!("\n\n[Recent messages by {me} — style examples]\n"));
        s.push_str(&samples.iter().map(|x| format!("- {x}")).collect::<Vec<_>>().join("\n"));
    }
    match brief {
        Some(b) => s.push_str(&format!("\n\n[Draft from {me} — keep its point and rewrite it as natural text]\n{b}")),
        None => s.push_str("\n\n(No draft: write the next line that fits the flow of the conversation)"),
    }
    let guide: String = cfg.guide.trim().chars().take(MAX_GUIDE_CHARS).collect();
    if !guide.is_empty() {
        s.push_str(&format!("\n\n[Additional guidance]\n{guide}"));
    }
    s
}

/// 만들어진 프롬프트의 끝을 대필용으로 마무리: 프리셋의 끝 assistant prefill을 떼고(상대 캐릭터로 이어 쓰게 만듦) 지시를 **맨 끝 user 메시지**로 붙인다 —
/// 프리셋의 뒤쪽 블록(지침·jailbreak)이 마지막 말이 되어 지시를 덮지 못하게. 응답 한도는 길이 옵션으로
pub fn finish_prompt(prompt: &mut crate::models::ai_service::PromptMessage, instruction: String, length: &str, stream: bool) {
    use crate::models::ai_service::{PromptMessageItem, PromptRole};
    while prompt.messages.last().is_some_and(|m| m.role == PromptRole::Assistant) {
        prompt.messages.pop();
    }
    prompt.messages.push(PromptMessageItem {
        role: PromptRole::User,
        content: instruction,
        token: 0,
        images: vec![],
    });
    prompt.chat_options.max_tokens = Some(max_tokens(length));
    prompt.chat_options.stream = stream;
}

fn strip_label(line: &str, name: &str) -> Option<String> {
    let t = line.trim_start().trim_start_matches(['*', '_', '[']);
    let rest = t.strip_prefix(name)?;
    let rest = rest.trim_start_matches(['*', '_', ']']).trim_start();
    let rest = rest.strip_prefix(':').or_else(|| rest.strip_prefix('：'))?;
    Some(rest.trim_start().to_string())
}

fn unwrap_quotes(s: &str) -> &str {
    for (a, b) in [('"', '"'), ('“', '”'), ('「', '」'), ('\'', '\'')] {
        if s.chars().count() >= 2 && s.starts_with(a) && s.ends_with(b) && !s[a.len_utf8()..s.len() - b.len_utf8()].contains(a) {
            return s[a.len_utf8()..s.len() - b.len_utf8()].trim();
        }
    }
    s
}

/// 결과 정리: 앞의 "내 이름:" 말머리·감싼 따옴표를 벗기고, 상대 이름으로 시작하는 줄부터는 잘라 낸다(AI가 상대 몫까지 이어 쓴 것). 비면 None
pub fn clean_output(text: &str, me: &str, other: &str) -> Option<String> {
    let mut lines: Vec<String> = vec![];
    for (i, line) in text.trim().lines().enumerate() {
        let mine = if i == 0 { strip_label(line, me) } else { None };
        let line = mine.unwrap_or_else(|| line.to_string());
        if !other.trim().is_empty() && strip_label(&line, other).is_some() {
            break; // 상대의 말을 쓰기 시작함 — 여기서부터 버림
        }
        lines.push(line);
    }
    let joined = lines.join("\n");
    let t = unwrap_quotes(joined.trim());
    let t: String = t.chars().take(MAX_OUTPUT_CHARS).collect();
    let t = t.trim().to_string();
    (!t.is_empty()).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_puts_style_samples_draft_and_guide_in_order() {
        let cfg = GhostSetting {
            guide: "1인칭, 대사 위주".into(),
            ..Default::default()
        };
        let i = build_instruction("민", "아리아", Some("내일 같이 가자고 말하기"), &["안녕, 오늘 어때?".into()], &cfg);
        assert!(i.starts_with("[Ghostwrite request]") && i.contains("Do not write 아리아's words") && i.contains("three or four sentences"));
        let (a, b, c, d) = (
            i.find("style examples").unwrap(),
            i.find("안녕, 오늘 어때?").unwrap(),
            i.find("[Draft from").unwrap(),
            i.find("[Additional guidance]").unwrap(),
        );
        assert!(a < b && b < c && c < d, "{i}");
        assert!(i.contains("내일 같이 가자고 말하기") && i.contains("1인칭, 대사 위주"));
        // 초안이 없으면 흐름에 맞춰 만든다 / 길이 옵션
        let none = build_instruction(
            "민",
            "아리아",
            None,
            &[],
            &GhostSetting {
                length: "short".into(),
                ..Default::default()
            },
        );
        assert!(
            none.contains("No draft") && none.contains("one or two sentences") && !none.contains("style examples") && !none.contains("[Additional guidance]")
        );
        assert_eq!(
            (max_tokens("short"), max_tokens("medium"), max_tokens("long"), max_tokens("?")),
            (512, 768, 1024, 768)
        );
    }

    #[test]
    fn the_ghost_block_is_always_the_last_message_and_prefill_is_dropped() {
        use crate::models::ai_service::{ChatOptions, PromptMessage, PromptMessageItem, PromptRole};
        let item = |role, c: &str| PromptMessageItem {
            role,
            content: c.into(),
            token: 0,
            images: vec![],
        };
        let mut p = PromptMessage {
            chat_options: ChatOptions {
                max_tokens: Some(4096),
                stream: false,
                ..Default::default()
            },
            messages: vec![
                item(PromptRole::System, "규칙"),
                item(PromptRole::User, "안녕"),
                item(PromptRole::Assistant, "응"),
                item(PromptRole::System, "항상 {{char}}로 답하라"),
                item(PromptRole::Assistant, "{{char}}:"),
            ],
        };
        finish_prompt(&mut p, "[대필 요청] …".into(), "long", true);
        let roles: Vec<_> = p.messages.iter().map(|m| (m.role, m.content.as_str())).collect();
        assert_eq!(roles.last().unwrap(), &(PromptRole::User, "[대필 요청] …"), "지시가 마지막 말");
        assert!(!p.messages.iter().any(|m| m.content == "{{char}}:"), "끝의 prefill은 뗌");
        assert!(p.messages.iter().any(|m| m.content.contains("항상")), "그 앞의 프리셋 블록은 그대로");
        assert_eq!(
            (p.chat_options.max_tokens, p.chat_options.stream),
            (Some(1024), true),
            "한도는 프리셋이 아니라 길이 옵션으로"
        );
    }

    #[test]
    fn brief_and_samples_are_capped_and_cleaned() {
        assert_eq!(clean_brief("  가자  ").as_deref(), Some("가자"));
        assert_eq!(clean_brief("  \n"), None);
        assert_eq!(clean_brief(&"가".repeat(5000)).unwrap().chars().count(), MAX_BRIEF_CHARS);
        let h: Vec<(String, String)> = vec![
            ("user".into(), "첫 번째".into()),
            ("assistant".into(), "AI 말".into()),
            ("user".into(), "<b>두</b> 번째".into()),
            ("user".into(), "세 번째".into()),
            ("user".into(), "네 번째".into()),
        ];
        assert_eq!(style_samples(&h), vec!["두 번째", "세 번째", "네 번째"], "최근 3개, 사용자의 것만, 태그 제거");
        assert!(style_samples(&[("assistant".into(), "x".into())]).is_empty());
    }

    #[test]
    fn output_loses_labels_quotes_and_the_other_sides_lines() {
        assert_eq!(clean_output("민: 같이 가자.", "민", "아리아").as_deref(), Some("같이 가자."));
        assert_eq!(clean_output("**민**: “좋아, 가자.”", "민", "아리아").as_deref(), Some("좋아, 가자."));
        assert_eq!(clean_output("\"그래, 가자\"", "민", "아리아").as_deref(), Some("그래, 가자"));
        assert_eq!(
            clean_output("그래, 가자.\n아리아: 좋아요!\n(아리아가 웃었다)", "민", "아리아").as_deref(),
            Some("그래, 가자."),
            "상대 몫은 버림"
        );
        assert_eq!(
            clean_output("그래, \"가자\" 하고 말했다", "민", "아리아").as_deref(),
            Some("그래, \"가자\" 하고 말했다"),
            "안쪽 따옴표는 그대로"
        );
        assert_eq!(
            clean_output("아리아: 내가 먼저 말할게", "민", "아리아"),
            None,
            "처음부터 상대 말이면 비었다고 봄"
        );
        assert_eq!(clean_output("  \n ", "민", "아리아"), None);
        assert_eq!(clean_output(&"가".repeat(5000), "민", "아리아").unwrap().chars().count(), MAX_OUTPUT_CHARS);
    }
}
