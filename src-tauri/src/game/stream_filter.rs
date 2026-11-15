//! 스트리밍 중인 AI 응답에서 연출·상태 태그를 걸러 화면에 보여줄 글만 내보낸다.
//!
//! 태그(`<choice>…</choice>`, `<emotion::이름>기쁨</emotion>`, `<updateVar>…</updateVar>` 등)는 조각 중간에서
//! 잘려 오므로, `<`를 만나면 태그인지 판단될 때까지 붙잡아 둔다. 여는 태그 안의 내용은 닫는 태그까지 숨긴다.
//! 저장·연출에 쓰는 최종 결과는 여전히 완성된 응답을 파싱한 것이고, 여기서 나온 글은 기다리는 동안의 미리보기다.

/// 태그로 보기에는 너무 긴 `<…` (그냥 글자로 내보낸다)
const MAX_TAG_LEN: usize = 64;

#[derive(Default)]
pub struct StreamTagFilter {
    /// 태그인지 아직 모르는 `<…`
    pending: String,
    /// 여는 태그를 지나 닫는 태그를 기다리는 중
    hidden: bool,
}

impl StreamTagFilter {
    /// 조각 하나를 넣고, 지금 보여줘도 되는 글을 돌려준다.
    pub fn push(&mut self, delta: &str) -> String {
        let mut out = String::new();
        for c in delta.chars() {
            if self.pending.is_empty() {
                if c == '<' {
                    self.pending.push(c);
                } else if !self.hidden {
                    out.push(c);
                }
                continue;
            }

            self.pending.push(c);
            // `<` 다음 글자가 영문자나 `/`가 아니면 태그가 아니다 ("<3", "< 이건")
            if self.pending.chars().count() == 2 && !(c.is_ascii_alphabetic() || c == '/') {
                self.flush_literal(&mut out);
                continue;
            }
            if c == '>' {
                let closing = self.pending.starts_with("</");
                let self_closing = self.pending.ends_with("/>");
                self.pending.clear();
                if closing {
                    self.hidden = false;
                } else if !self_closing {
                    self.hidden = true;
                }
                continue;
            }
            if self.pending.len() > MAX_TAG_LEN {
                self.flush_literal(&mut out);
            }
        }
        out
    }

    fn flush_literal(&mut self, out: &mut String) {
        let pending = std::mem::take(&mut self.pending);
        if !self.hidden {
            out.push_str(&pending);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(chunks: &[&str]) -> String {
        let mut f = StreamTagFilter::default();
        chunks.iter().map(|c| f.push(c)).collect()
    }

    #[test]
    fn hides_tags_split_across_chunks() {
        assert_eq!(run(&["미나: 안녕", "<emo", "tion::미나>기", "쁨</emo", "tion> 반가워"]), "미나: 안녕 반가워");
        assert_eq!(run(&["좋아<choice>같이 가자</choice>", "<choice>싫어</choice>"]), "좋아");
    }

    #[test]
    fn keeps_text_that_only_looks_like_a_tag() {
        assert_eq!(run(&["사랑해 <3", " 진짜로"]), "사랑해 <3 진짜로");
        assert_eq!(run(&["a < b"]), "a < b");
        let long = format!("<{}", "x".repeat(80));
        assert_eq!(run(&[&long]), long);
    }

    #[test]
    fn self_closing_tag_does_not_hide_following_text() {
        assert_eq!(run(&["앞<br/>뒤"]), "앞뒤");
    }
}
