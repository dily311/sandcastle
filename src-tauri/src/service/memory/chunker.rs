//! chunk 만들기 (CLAUDE.md 3-C, ADR-015). 순수 함수 — DB·네트워크 없음.
//!
//! chunk = 연속한 보이는 메시지 묶음. 경계는 **메시지 id에 고정**한다: 목표 글자 수에 닿아 닫힌 chunk는 다시 색인해도
//! 경계를 옮기지 않아서, 메시지를 덧붙여도 앞 chunk의 해시가 그대로이고 고친 메시지의 chunk만 다시 임베딩한다.
use sha2::{Digest, Sha256};

/// 규칙(글 뽑기·경계)을 바꾸면 올린다 — 해시가 달라져 옛 벡터를 쓰지 않는다
pub const CHUNKER_VERSION: u32 = 1;
/// 닫는 목표 글자 수 (한국어 대화 기준)
pub const TARGET_CHARS: usize = 1000;
/// 한 메시지에서 쓰는 최대 글자 수 (임베딩 입력 한도)
pub const MAX_MESSAGE_CHARS: usize = 4000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemMsg {
    pub id: String,
    pub role: String,
    pub name: Option<String>,
    pub content: String,
    /// AI에게 숨긴 메시지
    pub hidden: bool,
}

/// 이미 색인된 chunk의 경계 (같은 대화·같은 모델의 표 행)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Existing {
    pub first_message_id: String,
    pub last_message_id: String,
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub first_message_id: String,
    pub last_message_id: String,
    /// 임베딩할 글 (앞 메시지 한 줄 + chunk 줄들)
    pub text: String,
    /// 접두를 뺀 chunk 줄들 (결과에 보여 줄 글)
    pub body: String,
    pub text_hash: String,
    pub closed: bool,
}

/// 색인할 메시지인가: 사용자·AI 말이고 숨기지 않았고 뽑은 글이 비지 않음
pub fn indexable(m: &MemMsg) -> bool {
    (m.role == "user" || m.role == "assistant") && !m.hidden && !clean(&m.content).is_empty()
}

/// 코드 블록·HTML 태그·앱 태그(<image>·<cg>)를 빼고 공백을 정리한다
pub fn clean(text: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    for line in text.split('\n') {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut rest = line;
        let mut plain = String::new();
        while let Some(i) = rest.find('<') {
            plain.push_str(&rest[..i]);
            let after = &rest[i + 1..];
            // 짧은 태그만 태그로 본다 ("3 < 5" 같은 글을 지우지 않게)
            match after.find('>') {
                Some(j) if j <= 300 && after.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '/' || c == '!') => rest = &after[j + 1..],
                _ => {
                    plain.push('<');
                    rest = after;
                }
            }
        }
        plain.push_str(rest);
        out.push_str(&plain);
        out.push('\n');
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn line_of(m: &MemMsg) -> String {
    let who = m
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(if m.role == "user" { "사용자" } else { "AI" });
    let body: String = clean(&m.content).chars().take(MAX_MESSAGE_CHARS).collect();
    format!("{who}: {body}")
}

pub fn hash_text(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(CHUNKER_VERSION.to_le_bytes());
    h.update(text.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn build(msgs: &[&MemMsg], from: usize, to: usize, closed: bool) -> Planned {
    // 접두: 바로 앞 메시지 한 줄 (문맥용 — 경계는 옮기지 않는다)
    let body = msgs[from..=to].iter().map(|m| line_of(m)).collect::<Vec<_>>().join(
        "
",
    );
    let text = if from > 0 {
        format!(
            "{}
{body}",
            line_of(msgs[from - 1])
        )
    } else {
        body.clone()
    };
    Planned {
        first_message_id: msgs[from].id.clone(),
        last_message_id: msgs[to].id.clone(),
        text_hash: hash_text(&text),
        body,
        text,
        closed,
    }
}

/// 대화 전체(메시지 순서대로)와 이미 있는 chunk 경계로 chunk 계획을 만든다.
/// 닫힌 기존 chunk의 경계는 그대로 두고(양 끝 메시지가 아직 있고 순서·겹침이 맞을 때만), 나머지는 목표 글자 수까지 모아 닫는다.
/// 끝에서 목표에 못 미친 것은 열린 chunk.
pub fn plan(all: &[MemMsg], existing: &[Existing]) -> Vec<Planned> {
    let msgs: Vec<&MemMsg> = all.iter().filter(|m| indexable(m)).collect();
    let pos = |id: &str| msgs.iter().position(|m| m.id == id);
    // 고정할 경계: 닫혔고, 양 끝이 있고, 앞에서부터 순서대로 겹치지 않는 것
    let mut sorted: Vec<(usize, usize)> = existing
        .iter()
        .filter(|e| e.closed)
        .filter_map(|e| Some((pos(&e.first_message_id)?, pos(&e.last_message_id)?)))
        .filter(|(a, b)| a <= b)
        .collect();
    sorted.sort_unstable();
    let mut fixed: Vec<(usize, usize)> = vec![];
    for (a, b) in sorted {
        if fixed.last().is_none_or(|(_, last)| a > *last) {
            fixed.push((a, b));
        }
    }
    let mut out = vec![];
    let mut i = 0;
    while i < msgs.len() {
        if let Some(&(a, b)) = fixed.iter().find(|(a, _)| *a == i) {
            out.push(build(&msgs, a, b, true));
            i = b + 1;
            continue;
        }
        let next_fixed = fixed.iter().map(|(a, _)| *a).find(|a| *a > i).unwrap_or(msgs.len());
        let (mut j, mut chars) = (i, 0usize);
        let closed;
        loop {
            chars += line_of(msgs[j]).chars().count();
            if chars >= TARGET_CHARS {
                closed = true;
                break;
            }
            if j + 1 >= next_fixed {
                // 다음 고정 chunk 앞에서 끝나면 그 경계가 확정이므로 닫힌 것으로
                closed = next_fixed < msgs.len();
                break;
            }
            j += 1;
        }
        out.push(build(&msgs, i, j, closed));
        i = j + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, role: &str, text: &str) -> MemMsg {
        MemMsg {
            id: id.into(),
            role: role.into(),
            name: None,
            content: text.into(),
            hidden: false,
        }
    }
    fn long(n: usize) -> String {
        "가".repeat(n)
    }
    fn conv(n: usize, len: usize) -> Vec<MemMsg> {
        (0..n)
            .map(|i| m(&format!("m{i}"), if i % 2 == 0 { "user" } else { "assistant" }, &long(len)))
            .collect()
    }
    fn ids(p: &[Planned]) -> Vec<(String, String, bool)> {
        p.iter().map(|c| (c.first_message_id.clone(), c.last_message_id.clone(), c.closed)).collect()
    }
    fn existing(p: &[Planned]) -> Vec<Existing> {
        p.iter()
            .map(|c| Existing {
                first_message_id: c.first_message_id.clone(),
                last_message_id: c.last_message_id.clone(),
                closed: c.closed,
            })
            .collect()
    }

    #[test]
    fn clean_strips_markup_and_keeps_plain_angle_brackets() {
        assert_eq!(
            clean("안녕 <b>세계</b>\n```html\n<div>x</div>\n```\n<image>그림</image>끝 3 < 5"),
            "안녕 세계 그림끝 3 < 5"
        );
        assert!(!indexable(&m("a", "assistant", "```\ncode\n```")), "코드뿐이면 색인하지 않음");
        assert!(
            !indexable(&MemMsg {
                hidden: true,
                ..m("a", "user", "안녕")
            }) && !indexable(&m("a", "system", "안녕"))
        );
    }

    #[test]
    fn greedy_chunks_close_at_the_target_and_the_tail_stays_open() {
        let p = plan(&conv(5, 400), &[]);
        // 줄 하나가 약 405자 → 3개에서 닫힘, 나머지 2개는 목표 미달이라 열림
        assert_eq!(ids(&p), vec![("m0".into(), "m2".into(), true), ("m3".into(), "m4".into(), false)]);
        // 접두: 둘째 chunk는 앞 메시지(m2) 한 줄을 붙여 임베딩
        assert_eq!((p[0].text.lines().count(), p[1].text.lines().count()), (3, 3));
        assert_eq!(p[1].text_hash.len(), 64);
    }

    #[test]
    fn appending_messages_keeps_closed_chunks_and_extends_the_open_one() {
        let first = conv(5, 400);
        let p1 = plan(&first, &[]);
        let mut more = first.clone();
        more.extend((5..9).map(|i| m(&format!("m{i}"), "user", &long(400))));
        let p2 = plan(&more, &existing(&p1));
        assert_eq!(p2[0], p1[0], "닫힌 chunk는 글·해시까지 그대로");
        assert_eq!(p2[1].first_message_id, "m3");
        assert_ne!(p2[1].text_hash, p1[1].text_hash, "열린 chunk는 이어 붙으면 새 해시");
        assert!(p2[1].closed && p2.len() == 3);
    }

    #[test]
    fn editing_a_message_changes_only_its_own_chunk() {
        let msgs = conv(9, 400);
        let p1 = plan(&msgs, &[]);
        assert_eq!(p1.len(), 3);
        let mut edited = msgs.clone();
        edited[4].content = long(400) + "고침";
        let p2 = plan(&edited, &existing(&p1));
        assert_eq!(ids(&p2), ids(&p1), "경계는 그대로");
        let changed: Vec<bool> = p1.iter().zip(&p2).map(|(a, b)| a.text_hash != b.text_hash).collect();
        assert_eq!(changed, vec![false, true, false]);
    }

    #[test]
    fn deleted_boundary_message_re_chunks_only_that_part() {
        let msgs = conv(7, 400);
        let p1 = plan(&msgs, &[]);
        let mut gone = msgs.clone();
        gone.remove(2); // 첫 chunk의 끝 메시지
        let p2 = plan(&gone, &existing(&p1));
        assert_eq!(
            ids(&p2),
            vec![
                ("m0".into(), "m1".into(), true),
                ("m3".into(), "m5".into(), true),
                ("m6".into(), "m6".into(), false)
            ]
        );
        assert_ne!(p2[1].text_hash, p1[1].text_hash, "접두 줄이 m2에서 m1로 바뀌어 해시도 바뀜");
    }

    #[test]
    fn hidden_and_non_dialogue_messages_are_skipped_and_long_ones_cut() {
        let mut msgs = conv(3, 100);
        msgs.insert(
            1,
            MemMsg {
                hidden: true,
                ..m("h", "assistant", "숨김")
            },
        );
        msgs.insert(2, m("s", "system", "규칙"));
        let p = plan(&msgs, &[]);
        assert_eq!(p.len(), 1);
        assert!(!p[0].text.contains("숨김") && !p[0].text.contains("규칙"));
        let one = plan(&[m("x", "user", &long(9000))], &[]);
        assert!(one[0].text.chars().count() <= MAX_MESSAGE_CHARS + 8);
    }

    #[test]
    fn hash_depends_on_text() {
        assert_eq!(hash_text("a"), hash_text("a"));
        assert_ne!(hash_text("a"), hash_text("b"));
    }
}
