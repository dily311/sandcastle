//! 메시지 linter (CLAUDE.md 3-B, ADR-014). 응답이 저장되기 직전에, 요금 없이 앱 안에서만 돈다.
//!
//! 순서: AI 응답 → 정규식(AiOutput) → <image>·<cg> 정리 → **linter** → 저장 → (화면) 커뮤니티 플러그인 후처리.
//! 그래서 플러그인은 늘 linter가 본 뒤의 글을 받는다. 스트리밍 중에는 돌지 않는다(저장할 때 한 번).
//!
//! 규칙마다 끔 / 표시만 / 자동 고침. 고쳤으면 호출한 쪽이 meta.lint에 원문을 남긴다.
//! 코드 블록(```) 안은 검사하지 않는다 — 코드의 `*`·`"`·`<`는 글이 아니므로.
use serde::{Deserialize, Serialize};

use crate::models::settings::{LintMode, LintSetting};

/// 검사 결과 한 건
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// unclosed / repeat / stuck / impersonate / banned / length
    pub rule: String,
    /// info / warn
    pub level: String,
    pub text: String,
    /// 이 건을 글에서 고쳤나
    pub fixed: bool,
    /// 규칙이 알려 주는 상세: banned는 걸린 단어들(\u{1f}로 이음), stuck은 반복된 말(20자까지) — 자동 재생성 지시를 만들 때 쓴다
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintOutcome {
    /// 고친 뒤의 글 (고치지 않았으면 입력 그대로)
    pub content: String,
    pub findings: Vec<Finding>,
}

impl LintOutcome {
    pub fn changed(&self, original: &str) -> bool {
        self.content != original
    }
}

/// 짝이 있는 HTML 태그만 본다 (`<br>`·`<img>` 같은 빈 태그와 `<cg>`·`<image>` 같은 앱 태그는 보지 않음)
const PAIRED_TAGS: &[&str] = &[
    "b",
    "i",
    "u",
    "s",
    "em",
    "strong",
    "small",
    "mark",
    "sub",
    "sup",
    "code",
    "pre",
    "span",
    "div",
    "p",
    "center",
    "font",
    "a",
    "ul",
    "ol",
    "li",
    "table",
    "thead",
    "tbody",
    "tr",
    "td",
    "th",
    "details",
    "summary",
    "blockquote",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "style",
];

fn finding(rule: &str, level: &str, text: impl Into<String>, fixed: bool) -> Finding {
    Finding {
        rule: rule.into(),
        level: level.into(),
        text: text.into(),
        fixed,
        detail: None,
    }
}

fn finding_d(rule: &str, level: &str, text: impl Into<String>, fixed: bool, detail: String) -> Finding {
    Finding {
        detail: Some(detail),
        ..finding(rule, level, text, fixed)
    }
}

/// 코드 펜스(```) 바깥의 글, 그리고 펜스가 닫히지 않았는지
fn outside_fences(text: &str) -> (String, bool) {
    let mut out = String::new();
    let mut in_fence = false;
    for line in text.split('\n') {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            out.push_str(line);
            out.push('\n');
        }
    }
    (out, in_fence)
}

/// 한 태그의 (닫는 태그인가, 이름, 끝 위치). `<`로 시작해 `>`로 끝나는 짧은 태그만
fn parse_tag(s: &str) -> Option<(bool, String, usize)> {
    let rest = s.strip_prefix('<')?;
    let end = rest.find('>')?;
    if end > 300 || rest[..end].contains('<') {
        return None;
    }
    let inner = &rest[..end];
    let (closing, body) = match inner.strip_prefix('/') {
        Some(b) => (true, b),
        None => (false, inner),
    };
    if body.ends_with('/') {
        return None; // 자기 닫음
    }
    let name: String = body.chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase();
    if name.is_empty() || !body.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    Some((closing, name, end + 2))
}

/// (열린 채 남은 태그 이름들 — 안쪽부터 닫을 순서, 짝 없는 닫는 태그들, 태그를 뺀 글)
fn scan_tags(text: &str) -> (Vec<String>, Vec<String>, String) {
    let mut stack: Vec<String> = vec![];
    let mut stray = vec![];
    let mut plain = String::new();
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        if rest.starts_with('<') {
            if let Some((closing, name, len)) = parse_tag(rest) {
                i += len;
                if PAIRED_TAGS.contains(&name.as_str()) {
                    if !closing {
                        stack.push(name);
                    } else if let Some(pos) = stack.iter().rposition(|t| *t == name) {
                        stack.truncate(pos);
                    } else {
                        stray.push(name);
                    }
                }
                plain.push(' ');
                continue;
            }
        }
        let ch = rest.chars().next().expect("남은 글이 있음");
        plain.push(ch);
        i += ch.len_utf8();
    }
    stack.reverse();
    (stack, stray, plain)
}

/// `` `코드` `` 조각을 뺀다 (한 줄 안에서 짝이 맞는 것만)
fn strip_inline_code(text: &str) -> String {
    text.split('\n')
        .map(|line| {
            let parts: Vec<&str> = line.split('`').collect();
            if parts.len() < 3 {
                return line.to_string();
            }
            // 짝수 번째 조각이 코드. 짝이 안 맞는 마지막 백틱은 글로 둔다
            let pairs = (parts.len() - 1) / 2;
            let mut s = String::new();
            for (n, p) in parts.iter().enumerate() {
                if n % 2 == 0 || n > pairs * 2 {
                    s.push_str(p);
                    if n > pairs * 2 && n + 1 < parts.len() {
                        s.push('`');
                    }
                }
            }
            s
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 줄 머리의 목록 표시(`* 항목`)가 아닌 `*` 개수 (`**`는 따로 센다)
fn count_stars(text: &str) -> (usize, usize) {
    let (mut double, mut single) = (0, 0);
    for line in text.split('\n') {
        let t = line.trim_start();
        let body = t.strip_prefix("* ").unwrap_or(line);
        let b: Vec<char> = body.chars().collect();
        let mut i = 0;
        while i < b.len() {
            if b[i] == '*' {
                if i + 1 < b.len() && b[i + 1] == '*' {
                    double += 1;
                    i += 2;
                    continue;
                }
                single += 1;
            }
            i += 1;
        }
    }
    (double, single)
}

fn check_unclosed(text: &str, mode: LintMode, findings: &mut Vec<Finding>) -> String {
    let fix = mode == LintMode::Fix;
    let (outside, fence_open) = outside_fences(text);
    let (open_tags, stray, plain) = scan_tags(&outside);
    let plain = strip_inline_code(&plain);
    let mut tail = String::new();

    if fence_open {
        findings.push(finding("unclosed", "warn", "코드 블록(```)이 닫히지 않음", fix));
        if fix {
            tail.push_str("\n```");
        }
    }
    if !open_tags.is_empty() {
        findings.push(finding(
            "unclosed",
            "warn",
            format!("닫히지 않은 태그: {}", open_tags.iter().map(|t| format!("<{t}>")).collect::<Vec<_>>().join(" ")),
            fix,
        ));
        if fix {
            for t in &open_tags {
                tail.push_str(&format!("</{t}>"));
            }
        }
    }
    for t in &stray {
        findings.push(finding("unclosed", "info", format!("짝 없는 닫는 태그: </{t}>"), false));
    }
    let (double, single) = count_stars(&plain);
    if double % 2 == 1 {
        findings.push(finding("unclosed", "warn", "굵게(**)가 닫히지 않음", fix));
        if fix {
            tail.push_str("**");
        }
    }
    if single % 2 == 1 {
        findings.push(finding("unclosed", "warn", "기울임(*)이 닫히지 않음", fix));
        if fix {
            tail.push('*');
        }
    }
    let straight = plain.chars().filter(|c| *c == '"').count();
    if straight % 2 == 1 {
        findings.push(finding("unclosed", "warn", "따옴표(\")가 닫히지 않음", fix));
        if fix {
            tail.push('"');
        }
    }
    let (lo, lc) = (plain.chars().filter(|c| *c == '“').count(), plain.chars().filter(|c| *c == '”').count());
    if lo > lc {
        findings.push(finding("unclosed", "warn", "여는 따옴표(“)가 닫히지 않음", fix));
        if fix {
            tail.push('”');
        }
    }
    tail
}

fn normalize(p: &str) -> String {
    p.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 문단 반복. 고치면 두 번째부터 지운다. 코드 블록이 걸친 문단은 건드리지 않는다
fn check_repeat(text: &str, mode: LintMode, findings: &mut Vec<Finding>) -> Option<String> {
    if text.contains("```") {
        return None;
    }
    let paras: Vec<&str> = text.split("\n\n").collect();
    let mut seen = std::collections::HashSet::new();
    let mut keep: Vec<&str> = vec![];
    let mut dups = 0;
    for p in &paras {
        let n = normalize(p);
        if n.chars().count() >= 20 && !seen.insert(n) {
            dups += 1;
        } else {
            keep.push(p);
        }
    }
    if dups == 0 {
        return None;
    }
    let fix = mode == LintMode::Fix;
    findings.push(finding("repeat", "warn", format!("같은 문단이 {dups}번 더 반복됨"), fix));
    fix.then(|| keep.join("\n\n"))
}

/// 내 이름으로 시작하는 줄(`이름: …`)이 나오면 AI가 내 대사를 대신 쓴 것. 고치면 그 줄부터 잘라 낸다
fn check_impersonate(text: &str, user_name: &str, mode: LintMode, findings: &mut Vec<Finding>) -> Option<String> {
    let name = user_name.trim();
    if name.is_empty() {
        return None;
    }
    let mut in_fence = false;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if t.trim_start().starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence {
            let t = t.trim_start_matches(['*', '_', '[']);
            if let Some(rest) = t.strip_prefix(name) {
                let rest = rest.trim_start_matches(['*', '_', ']']).trim_start();
                if rest.starts_with(':') || rest.starts_with('：') {
                    let line_no = text[..offset].matches('\n').count() + 1;
                    let fix = mode == LintMode::Fix && !text[..offset].trim().is_empty();
                    findings.push(finding("impersonate", "warn", format!("{line_no}번째 줄부터 '{name}'의 대사를 대신 씀"), fix));
                    return fix.then(|| text[..offset].trim_end().to_string());
                }
            }
        }
        offset += line.len();
    }
    None
}

/// 단어가 나온 곳(글자 위치 범위). 영문·숫자로만 된 단어는 단어 경계에서만 (ai가 said에 걸리지 않게), 한글은 조사·어미가 붙으므로 부분 일치.
/// 대소문자는 가리지 않는다 (글자 수가 바뀌지 않게 글자마다 소문자로)
fn word_spans(chars: &[char], word: &str) -> Vec<(usize, usize)> {
    let low = |c: char| c.to_lowercase().next().unwrap_or(c);
    let w: Vec<char> = word.chars().map(low).collect();
    if w.is_empty() {
        return vec![];
    }
    let boundary = w.iter().all(|c| c.is_ascii_alphanumeric());
    let word_char = |c: char| c.is_ascii_alphanumeric();
    let mut out = vec![];
    let mut i = 0;
    while i + w.len() <= chars.len() {
        let hit = chars[i..i + w.len()].iter().map(|c| low(*c)).eq(w.iter().copied());
        let ok = hit && (!boundary || ((i == 0 || !word_char(chars[i - 1])) && (i + w.len() == chars.len() || !word_char(chars[i + w.len()]))));
        if ok {
            out.push((i, i + w.len()));
            i += w.len();
        } else {
            i += 1;
        }
    }
    out
}

fn check_banned(text: &str, words: &[String], mode: LintMode, findings: &mut Vec<Finding>) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut hits: Vec<(&str, Vec<(usize, usize)>)> = vec![];
    let mut seen = std::collections::HashSet::new();
    for w in words.iter().map(|w| w.trim()).filter(|w| !w.is_empty() && seen.insert(w.to_lowercase())) {
        let spans = word_spans(&chars, w);
        if !spans.is_empty() {
            hits.push((w, spans));
        }
    }
    if hits.is_empty() {
        return None;
    }
    let list = hits.iter().map(|(w, s)| format!("{w} ×{}", s.len())).collect::<Vec<_>>().join(", ");
    let detail = hits.iter().map(|(w, _)| *w).collect::<Vec<_>>().join("\u{1f}");
    let mut out = None;
    if mode == LintMode::Fix {
        let mut gone = vec![false; chars.len()];
        for (_, spans) in &hits {
            for (a, b) in spans {
                gone[*a..*b].iter_mut().for_each(|g| *g = true);
            }
        }
        let rest: String = chars.iter().zip(&gone).filter(|(_, g)| !**g).map(|(c, _)| *c).collect();
        if rest.trim().is_empty() {
            // 전부 지우면 빈 메시지가 됨 — 고치지 않고 표시만
            findings.push(finding_d(
                "banned",
                "warn",
                format!("금지어: {list} (모두 지우면 빈 글이라 표시만)"),
                false,
                detail,
            ));
            return None;
        }
        out = Some(rest);
    }
    findings.push(finding_d("banned", "warn", format!("금지어: {list}"), out.is_some(), detail));
    out
}

// ---------------------------------------------------------------------------
// 단어 반복 오류 (3-B-4): 모델이 같은 말을 끝없이 되풀이하는 고장. 문단이 겹치는 "반복 문단"과는 다르다
// ---------------------------------------------------------------------------

struct Tok {
    start: usize,
    end: usize,
    norm: String,
    ignorable: bool,
}

/// 공백으로 나눈 낱말 (코드 블록 안은 뺌). 구두점·숫자·이모지뿐인 것, 한 글자만 되풀이한 것(ㅋㅋㅋ·하하하·아아아), 태그 조각은 "무시"로 표시한다
fn tokenize(text: &str) -> Vec<Tok> {
    let mut out = vec![];
    let (mut in_fence, mut offset) = (false, 0usize);
    for line in text.split_inclusive('\n') {
        let base = offset;
        offset += line.len();
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        for tok in line.split_whitespace() {
            let start = base + (tok.as_ptr() as usize - line.as_ptr() as usize);
            let norm = tok.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
            let first = norm.chars().next();
            let ignorable =
                tok.contains('<') || tok.contains('>') || !norm.chars().any(char::is_alphabetic) || first.is_some_and(|f| norm.chars().all(|c| c == f));
            out.push(Tok {
                start,
                end: start + tok.len(),
                norm,
                ignorable,
            });
        }
    }
    out
}

#[derive(Debug, Clone)]
struct Run {
    /// 반복 구간의 바이트 범위
    start: usize,
    end: usize,
    /// 고칠 때 남기는 끝 (두 번까지)
    keep_end: usize,
    unit: String,
    count: usize,
}

/// 낱말 n개(1~4)가 연달아 되풀이: n=1은 8번, n≥2는 5번 이상 (응답이 한도에서 잘렸으면 30% 낮춤 — 진짜 반복은 대개 한도까지 간다)
fn token_runs(toks: &[Tok], low: bool) -> Vec<Run> {
    let (min1, minn) = if low { (6, 4) } else { (8, 5) };
    let mut taken = vec![false; toks.len()];
    let mut runs = vec![];
    for n in 1..=4usize {
        let min = if n == 1 { min1 } else { minn };
        let mut i = 0;
        while i + n <= toks.len() {
            let unit = &toks[i..i + n];
            let valid = !taken[i] && unit.iter().all(|t| !t.ignorable) && (n > 1 || unit[0].norm.chars().count() >= 2);
            if !valid {
                i += 1;
                continue;
            }
            let mut j = i + n;
            while j + n <= toks.len() && !taken[j] && toks[j..j + n].iter().zip(unit).all(|(a, b)| a.norm == b.norm) {
                j += n;
            }
            let count = (j - i) / n;
            if count >= min {
                taken[i..j].iter_mut().for_each(|t| *t = true);
                runs.push(Run {
                    start: unit[0].start,
                    end: toks[j - 1].end,
                    keep_end: toks[i + 2 * n - 1].end,
                    unit: unit.iter().map(|t| t.norm.as_str()).collect::<Vec<_>>().join(" "),
                    count,
                });
                i = j;
            } else {
                i += 1;
            }
        }
    }
    runs
}

/// 같은 문장이 연달아 3번 이상
fn sentence_runs(text: &str) -> Vec<Run> {
    if text.contains("```") {
        return vec![];
    }
    let mut sents: Vec<(usize, usize, String)> = vec![];
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if matches!(c, '.' | '!' | '?' | '。' | '…' | '\n') {
            let end = i + c.len_utf8();
            sents.push((start, end, text[start..end].split_whitespace().collect::<Vec<_>>().join(" ")));
            start = end;
        }
    }
    let mut runs = vec![];
    let mut i = 0;
    while i < sents.len() {
        let norm = &sents[i].2;
        if norm.chars().filter(|c| c.is_alphanumeric()).count() < 6 {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < sents.len() && sents[j].2 == *norm {
            j += 1;
        }
        if j - i >= 3 {
            runs.push(Run {
                start: sents[i].0,
                end: sents[j - 1].1,
                keep_end: sents[i + 1].1,
                unit: norm.clone(),
                count: j - i,
            });
            i = j;
        } else {
            i += 1;
        }
    }
    runs
}

/// 띄어쓰기 없이 2~10글자 덩어리가 연달아 10번 이상(40자 이상)
fn char_runs(text: &str, low: bool) -> Vec<Run> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let n = chars.len();
    let byte = |i: usize| if i < n { chars[i].0 } else { text.len() };
    let (min_count, min_len) = if low { (7, 28) } else { (10, 40) };
    let mut runs = vec![];
    let mut i = 0;
    while i < n {
        let mut found = None;
        for l in 2..=10usize {
            if i + 2 * l > n || chars[i].1 != chars[i + l].1 {
                continue;
            }
            let unit: Vec<char> = chars[i..i + l].iter().map(|c| c.1).collect();
            let distinct = unit.iter().collect::<std::collections::HashSet<_>>().len();
            if distinct < 2 || !unit.iter().any(|c| c.is_alphanumeric()) || unit.iter().any(|c| matches!(c, '<' | '>')) {
                continue;
            }
            let mut k = 1;
            while i + (k + 1) * l <= n && chars[i + k * l..i + (k + 1) * l].iter().map(|c| c.1).eq(unit.iter().copied()) {
                k += 1;
            }
            if k >= min_count && k * l >= min_len {
                found = Some((l, k, unit.iter().collect::<String>()));
                break;
            }
        }
        match found {
            Some((l, k, unit)) => {
                runs.push(Run {
                    start: byte(i),
                    end: byte(i + k * l),
                    keep_end: byte(i + 2 * l),
                    unit,
                    count: k,
                });
                i += k * l;
            }
            None => i += 1,
        }
    }
    runs
}

/// 반복 구간을 찾는다 (겹치면 앞의 것만)
fn find_stuck(text: &str, low: bool) -> Vec<Run> {
    fn merge(mut runs: Vec<Run>) -> Vec<Run> {
        runs.sort_by_key(|r| (r.start, std::cmp::Reverse(r.end)));
        let mut kept: Vec<Run> = vec![];
        for r in runs {
            if kept.last().is_none_or(|l| r.start >= l.end) {
                kept.push(r);
            }
        }
        kept
    }
    // 낱말·문장 단위를 먼저 (띄어쓰기 바로 앞 공백까지 끌어들이는 글자 단위보다 정확한 단위). 글자 단위는 그 밖에서만
    let mut kept = merge(token_runs(&tokenize(text), low).into_iter().chain(sentence_runs(text)).collect());
    let extra: Vec<Run> = char_runs(text, low)
        .into_iter()
        .filter(|c| kept.iter().all(|k| c.end <= k.start || c.start >= k.end))
        .collect();
    kept.extend(extra);
    merge(kept)
}

/// 고치면 반복 구간을 두 번까지만 남긴다
fn check_stuck(text: &str, mode: LintMode, hit_limit: bool, findings: &mut Vec<Finding>) -> Option<String> {
    let runs = find_stuck(text, hit_limit);
    let first = runs.first()?;
    let unit: String = first.unit.chars().take(20).collect();
    let mut out = text.to_string();
    for r in runs.iter().rev() {
        out.replace_range(r.keep_end..r.end, "");
    }
    let fixed = mode == LintMode::Fix && !out.trim().is_empty();
    let more = if runs.len() > 1 {
        format!(" 외 {}곳", runs.len() - 1)
    } else {
        String::new()
    };
    findings.push(finding_d(
        "stuck",
        "warn",
        format!("같은 말 '{unit}'이(가) {}번 연달아 반복됨{more}", first.count),
        fixed,
        unit,
    ));
    fixed.then_some(out)
}

fn check_length(text: &str, s: &LintSetting, findings: &mut Vec<Finding>) {
    let n = text.chars().count() as u32;
    if s.min_chars > 0 && n < s.min_chars {
        findings.push(finding("length", "info", format!("글이 짧음: {n}자 (최소 {}자)", s.min_chars), false));
    }
    if s.max_chars > 0 && n > s.max_chars {
        findings.push(finding("length", "info", format!("글이 김: {n}자 (최대 {}자)", s.max_chars), false));
    }
}

/// 응답 글을 검사하고 고칠 규칙은 고친다. `user_name`은 내 이름(페르소나)
/// 자동 재생성의 금지어 검사 (메시지 linter와 목록·규칙이 따로다). 걸린 것이 있으면 `rewriteBanned` 결과 하나 (글은 고치지 않음)
pub fn rewrite_banned_finding(text: &str, words: &[String]) -> Option<Finding> {
    let mut found = vec![];
    check_banned(text, words, LintMode::Report, &mut found);
    found.pop().map(|f| Finding {
        rule: "rewriteBanned".into(),
        text: f.text.replace("금지어", "재생성 금지어"),
        ..f
    })
}

/// meta.lint에 결과 하나를 더한다 (없으면 새로 만든다). 자동 재생성이 자기 검사 결과를 같은 자리에 남겨 화면·다시 쓰기 판정이 한 곳을 보게 한다
pub fn with_finding(meta: Option<serde_json::Value>, content: &str, finding: Finding) -> serde_json::Value {
    let mut meta = meta.unwrap_or_else(|| {
        serde_json::json!({
            "content": content,
            "findings": [],
            "fixedRules": [],
            "at": chrono::Utc::now().timestamp_millis(),
        })
    });
    if let Some(list) = meta.get_mut("findings").and_then(serde_json::Value::as_array_mut) {
        list.push(serde_json::to_value(&finding).unwrap_or_default());
    }
    meta
}

pub fn run(text: &str, user_name: &str, s: &LintSetting) -> LintOutcome {
    run_with(text, user_name, s, false)
}

/// `hit_limit`: 응답이 출력 한도에서 잘렸음 (반복 오류 판정을 조금 낮춘다)
pub fn run_with(text: &str, user_name: &str, s: &LintSetting, hit_limit: bool) -> LintOutcome {
    let mut findings = vec![];
    let mut content = text.to_string();
    // 글이 바뀌는 규칙을 먼저(길이는 마지막), 닫기는 다른 고침이 끝난 뒤에 (잘라 낸 글 기준으로 닫아야 하므로)
    if s.impersonate != LintMode::Off {
        if let Some(c) = check_impersonate(&content, user_name, s.impersonate, &mut findings) {
            content = c;
        }
    }
    if s.stuck != LintMode::Off {
        if let Some(c) = check_stuck(&content, s.stuck, hit_limit, &mut findings) {
            content = c;
        }
    }
    if s.repeat != LintMode::Off {
        if let Some(c) = check_repeat(&content, s.repeat, &mut findings) {
            content = c;
        }
    }
    if s.banned != LintMode::Off {
        if let Some(c) = check_banned(&content, &s.banned_words, s.banned, &mut findings) {
            content = c;
        }
    }
    if s.unclosed != LintMode::Off {
        let tail = check_unclosed(&content, s.unclosed, &mut findings);
        content.push_str(&tail);
    }
    if s.length != LintMode::Off {
        check_length(&content, s, &mut findings);
    }
    LintOutcome { content, findings }
}

// ---------------------------------------------------------------------------
// 저장 연결: meta.lint (원문·결과·고친 규칙) + 되돌리기
// ---------------------------------------------------------------------------

/// 응답 저장 직전에 부른다. 꺼져 있으면 글을 그대로 돌려주고 아무것도 읽지 않는다 (Lazy Initialization).
/// 돌려주는 값: (저장할 글, meta.lint에 넣을 값 — 걸린 것이 없으면 None)
pub fn apply_to_response(app: &tauri::AppHandle, text: &str, user_name: &str, hit_limit: bool) -> (String, Option<serde_json::Value>) {
    use tauri::Manager;
    if !crate::sys::plugins::is_enabled(app, crate::sys::plugins::id::LINT) {
        return (text.to_string(), None);
    }
    let Some(state) = app.try_state::<crate::sys::settings::SettingsState<crate::models::settings::PluginSetting>>() else {
        return (text.to_string(), None);
    };
    let Ok(setting) = state.0.lock().map(|s| s.lint.clone()) else {
        return (text.to_string(), None);
    };
    let outcome = run_with(text, user_name, &setting, hit_limit);
    let meta = meta_of(text, &outcome);
    (outcome.content, meta)
}

/// meta.lint: `original`은 고쳤을 때만. 화면은 `content`가 지금 글과 같을 때만 보여 준다(그 뒤 고쳤으면 숨김)
pub fn meta_of(original: &str, o: &LintOutcome) -> Option<serde_json::Value> {
    if o.findings.is_empty() {
        return None;
    }
    let fixed_rules: Vec<&str> = {
        let mut v: Vec<&str> = o.findings.iter().filter(|f| f.fixed).map(|f| f.rule.as_str()).collect();
        v.dedup();
        v
    };
    Some(serde_json::json!({
        "original": o.changed(original).then_some(original),
        "content": o.content,
        "findings": o.findings,
        "fixedRules": fixed_rules,
        "at": chrono::Utc::now().timestamp_millis(),
    }))
}

/// 되돌리기: linter가 고친 글이 그대로일 때만 원문으로. 그 뒤 플러그인이나 사용자가 또 고쳤으면 거절
pub async fn revert(db: &sqlx::SqlitePool, message_id: &str) -> crate::error::Result<()> {
    use crate::error::AppError;
    use serde_json::Value;
    let mut tx = db.begin().await?;
    let row: Option<(String, Option<String>)> = sqlx::query_as("SELECT content, meta FROM messages WHERE id = ?")
        .bind(message_id)
        .fetch_optional(&mut *tx)
        .await?;
    let Some((content, meta_raw)) = row else {
        return Err(AppError::not_found(format!("메시지가 없음: {message_id}")));
    };
    let mut meta: Value = meta_raw
        .as_deref()
        .and_then(|r| serde_json::from_str(r).ok())
        .filter(Value::is_object)
        .ok_or_else(|| AppError::validation("메시지 meta를 읽을 수 없음", None))?;
    let Some(l) = meta.get("lint").cloned() else {
        return Err(AppError::validation("linter가 고친 메시지가 아님", None));
    };
    let Some(original) = l.get("original").and_then(Value::as_str).map(str::to_string) else {
        return Err(AppError::validation("linter가 글을 고치지 않았음", None));
    };
    if l.get("content").and_then(Value::as_str) != Some(content.as_str()) {
        return Err(AppError::validation(
            "linter가 고친 뒤 메시지가 또 바뀌어 되돌리지 않음 (플러그인 후처리를 먼저 되돌리세요)",
            None,
        ));
    }
    if let Some(m) = meta.as_object_mut() {
        m.remove("lint");
    }
    let sid = meta.get("swipeId").and_then(Value::as_u64).unwrap_or(0) as usize;
    if let Some(s) = meta.get_mut("swipes").and_then(Value::as_array_mut).and_then(|a| a.get_mut(sid)) {
        if s.get("content").and_then(Value::as_str) == Some(content.as_str()) {
            s["content"] = Value::String(original.clone());
        }
    }
    sqlx::query("UPDATE messages SET content = ?, meta = ?, updated_at = ? WHERE id = ?")
        .bind(&original)
        .bind(meta.to_string())
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(message_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn only(f: impl Fn(&mut LintSetting)) -> LintSetting {
        let mut s = LintSetting {
            unclosed: LintMode::Off,
            repeat: LintMode::Off,
            impersonate: LintMode::Off,
            banned: LintMode::Off,
            length: LintMode::Off,
            ..Default::default()
        };
        f(&mut s);
        s
    }
    fn rules(o: &LintOutcome) -> Vec<&str> {
        o.findings.iter().map(|f| f.rule.as_str()).collect()
    }

    #[test]
    fn clean_text_passes_untouched() {
        let all = LintSetting {
            unclosed: LintMode::Fix,
            repeat: LintMode::Fix,
            impersonate: LintMode::Fix,
            ..Default::default()
        };
        let t = "\"안녕.\" *웃으며 손을 흔든다.*\n\n**중요**한 `a*b` 이야기.\n\n```\nlet x = \"a;\n```\n\n<b>굵게</b><br>끝";
        let o = run(t, "나", &all);
        assert!(o.findings.is_empty(), "{:?}", o.findings);
        assert_eq!(o.content, t);
    }

    #[test]
    fn unclosed_markup_is_reported_or_closed() {
        let s = only(|s| s.unclosed = LintMode::Report);
        let o = run("<div><b>굵게 \"말하다 *웃음 **강조", "나", &s);
        assert_eq!(rules(&o).len(), 4, "{:?}", o.findings);
        assert!(o.findings.iter().all(|f| !f.fixed) && o.content == "<div><b>굵게 \"말하다 *웃음 **강조");

        let s = only(|s| s.unclosed = LintMode::Fix);
        let o = run("<div><b>굵게 \"말하다 *웃음 **강조", "나", &s);
        assert!(o.findings.iter().all(|f| f.fixed));
        assert_eq!(o.content, "<div><b>굵게 \"말하다 *웃음 **강조</b></div>***\"", "{}", o.content);
        // 고친 글을 다시 검사하면 깨끗하다
        assert!(run(&o.content, "나", &s).findings.is_empty(), "{:?}", run(&o.content, "나", &s).findings);

        let o = run("앞\n```js\nlet a = 1;", "나", &s);
        assert_eq!(o.content, "앞\n```js\nlet a = 1;\n```");
    }

    #[test]
    fn code_html_attributes_lists_and_apps_tags_are_not_false_positives() {
        let s = only(|s| s.unclosed = LintMode::Fix);
        let t = "<div class=\"box\">\n* 항목 하나\n* 항목 둘\n</div>\n`\"` 는 따옴표\n<cg>smile</cg> <image>x</image>\n```\n<div> * \"\n```";
        let o = run(t, "나", &s);
        assert!(o.findings.is_empty(), "{:?}", o.findings);
        // 짝 없는 닫는 태그는 알리기만
        let o = run("끝</b>", "나", &s);
        assert_eq!((o.findings.len(), o.findings[0].fixed, o.content.as_str()), (1, false, "끝</b>"));
    }

    #[test]
    fn repeated_paragraphs() {
        let p = "같은 문장이 그대로 두 번 나오는 긴 문단입니다.";
        let t = format!("{p}\n\n다른 이야기.\n\n{p}");
        let o = run(&t, "나", &only(|s| s.repeat = LintMode::Report));
        assert_eq!((o.findings.len(), o.content.as_str()), (1, t.as_str()));
        let o = run(&t, "나", &only(|s| s.repeat = LintMode::Fix));
        assert_eq!(o.content, format!("{p}\n\n다른 이야기."));
        // 짧은 문단("네.")은 반복해도 센다 하지 않음
        assert!(run("네.\n\n네.", "나", &only(|s| s.repeat = LintMode::Fix)).findings.is_empty());
    }

    #[test]
    fn impersonation_is_cut_from_that_line() {
        let t = "민지가 웃었다.\n\n**나**: 그래, 가자.\n민지: 좋아.";
        let o = run(t, "나", &only(|s| s.impersonate = LintMode::Report));
        assert_eq!((o.findings.len(), o.content.as_str()), (1, t));
        assert!(o.findings[0].text.contains("3번째"));
        let o = run(t, "나", &only(|s| s.impersonate = LintMode::Fix));
        assert_eq!(o.content, "민지가 웃었다.");
        // 첫 줄부터면 비게 되므로 고치지 않음
        let o = run("나: 안녕", "나", &only(|s| s.impersonate = LintMode::Fix));
        assert_eq!((o.content.as_str(), o.findings[0].fixed), ("나: 안녕", false));
        // 이름이 다른 말머리·이름이 빈 경우
        assert!(run("나비: 안녕\n나는 간다", "나", &only(|s| s.impersonate = LintMode::Fix)).findings.is_empty());
        assert!(run("나: 안녕", "", &only(|s| s.impersonate = LintMode::Fix)).findings.is_empty());
    }

    #[test]
    fn banned_words_case_insensitive() {
        let s = |m| {
            only(|s| {
                s.banned = m;
                s.banned_words = vec!["AI".into(), " ".into(), "죄송".into()];
            })
        };
        let o = run("저는 ai라서 Ai처럼, 죄송합니다.", "나", &s(LintMode::Report));
        assert_eq!(o.findings.len(), 1);
        assert!(
            o.findings[0].text.contains("AI ×2") && o.findings[0].text.contains("죄송 ×1"),
            "{}",
            o.findings[0].text
        );
        let o = run("저는 ai라서 Ai처럼, 죄송합니다.", "나", &s(LintMode::Fix));
        assert_eq!(o.content, "저는 라서 처럼, 합니다.");
        // 전부 지워지면 고치지 않음
        let o = run("죄송", "나", &s(LintMode::Fix));
        assert_eq!((o.content.as_str(), o.findings[0].fixed), ("죄송", false));
    }

    #[test]
    fn length_is_reported_only_and_off_by_default() {
        let s = only(|s| {
            s.length = LintMode::Fix; // 고칠 수 없는 규칙이라 표시만
            s.min_chars = 5;
            s.max_chars = 10;
        });
        assert_eq!(run("가나", "나", &s).findings.len(), 1);
        assert_eq!(run("가나다라마바사아자차카", "나", &s).findings.len(), 1);
        let o = run("가나다라마바", "나", &s);
        assert!(o.findings.is_empty());
        assert!(run("가", "나", &LintSetting::default()).findings.is_empty(), "기본값은 길이 규칙 꺼짐");
    }

    #[test]
    fn fixes_compose_and_original_is_the_callers_to_keep() {
        let s = LintSetting {
            unclosed: LintMode::Fix,
            repeat: LintMode::Fix,
            impersonate: LintMode::Fix,
            ..Default::default()
        };
        let p = "반복되는 긴 문단이 여기에 한 번 더 나옵니다 정말로.";
        let t = format!("{p}\n\n{p}\n\n*웃었다\n나: 내 대사");
        let o = run(&t, "나", &s);
        assert_eq!(o.content, format!("{p}\n\n*웃었다*"));
        assert!(o.changed(&t) && o.findings.iter().all(|f| f.fixed));
    }

    #[tokio::test]
    async fn revert_restores_the_original_only_while_the_linted_text_stands() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::conn::init_pool(dir.path().join("a")).await.unwrap();
        sqlx::query("INSERT INTO conversations (id, title) VALUES ('c', 't')")
            .execute(&db)
            .await
            .unwrap();
        let s = LintSetting {
            unclosed: LintMode::Fix,
            ..Default::default()
        };
        let original = "*웃었다";
        let o = run(original, "나", &s);
        let lint = meta_of(original, &o).unwrap();
        assert_eq!((lint["original"].as_str(), lint["fixedRules"][0].as_str()), (Some("*웃었다"), Some("unclosed")));
        let meta = json!({ "lint": lint, "swipes": [{ "content": o.content }], "swipeId": 0 });
        sqlx::query("INSERT INTO messages (id, conversation_id, seq, role, content, meta) VALUES ('m', 'c', 1, 'assistant', ?, ?)")
            .bind(&o.content)
            .bind(meta.to_string())
            .execute(&db)
            .await
            .unwrap();
        // 표시만이거나 걸린 게 없으면 meta가 없다
        assert!(meta_of("괜찮음", &run("괜찮음", "나", &s)).is_none());
        // 그 뒤 글이 바뀌었으면 거절
        sqlx::query("UPDATE messages SET content = '다른 글' WHERE id = 'm'")
            .execute(&db)
            .await
            .unwrap();
        assert!(revert(&db, "m").await.is_err());
        sqlx::query("UPDATE messages SET content = ? WHERE id = 'm'")
            .bind(&o.content)
            .execute(&db)
            .await
            .unwrap();
        revert(&db, "m").await.unwrap();
        let (c, m): (String, String) = sqlx::query_as("SELECT content, meta FROM messages WHERE id = 'm'")
            .fetch_one(&db)
            .await
            .unwrap();
        let m: serde_json::Value = serde_json::from_str(&m).unwrap();
        assert_eq!(
            (c.as_str(), m.get("lint").is_none(), m["swipes"][0]["content"].as_str()),
            ("*웃었다", true, Some("*웃었다"))
        );
        assert!(revert(&db, "m").await.is_err(), "두 번 되돌리지 않음");
    }
}

#[cfg(test)]
mod stuck_tests {
    use super::*;

    fn only_stuck(mode: LintMode) -> LintSetting {
        LintSetting {
            unclosed: LintMode::Off,
            repeat: LintMode::Off,
            stuck: mode,
            impersonate: LintMode::Off,
            banned: LintMode::Off,
            length: LintMode::Off,
            ..Default::default()
        }
    }
    fn stuck(text: &str) -> Vec<Finding> {
        run(text, "나", &only_stuck(LintMode::Report)).findings
    }

    #[test]
    fn natural_repetition_is_not_a_loop() {
        for t in [
            "ㅋㅋㅋㅋㅋㅋㅋㅋㅋㅋㅋㅋ",
            "하하하하하하하하하하하하",
            "…………………………………………",
            "쿵쿵쿵쿵쿵쿵쿵쿵쿵쿵",
            "가자, 가자, 가자, 가자",
            "1. 항목\n2. 항목\n3. 항목\n4. 항목\n5. 항목\n6. 항목\n7. 항목\n8. 항목\n9. 항목",
            "아니, 아니야. 아니, 아니야.",
            "<br><br><br><br><br><br><br><br><br><br>",
            "```\n그래서 그래서 그래서 그래서 그래서 그래서 그래서 그래서 그래서 그래서\n```",
            "똑똑 똑똑 똑똑 똑똑 똑똑 똑똑", // 단어 하나가 아니라 같은 낱말 6번 — 8번 미만
        ] {
            assert!(stuck(t).is_empty(), "{t}: {:?}", stuck(t));
        }
    }

    #[test]
    fn degenerate_loops_are_caught_and_collapsed_to_two() {
        let f = stuck(&format!("그가 말했다. {}", "그래서 ".repeat(10)));
        assert_eq!((f.len(), f[0].rule.as_str(), f[0].detail.as_deref()), (1, "stuck", Some("그래서")), "{f:?}");
        assert!(f[0].text.contains("10번"));
        let two = stuck(&"아니 그래도 ".repeat(6));
        assert_eq!(two[0].detail.as_deref(), Some("아니 그래도"), "{two:?}");
        assert_eq!(stuck(&"나는 집에 갔다. ".repeat(4))[0].detail.as_deref(), Some("나는 집에 갔다."));
        assert_eq!(stuck(&"하나둘셋넷".repeat(12))[0].rule, "stuck", "띄어쓰기 없는 반복");

        let o = run(&format!("그가 말했다. {}끝", "그래서 ".repeat(10)), "나", &only_stuck(LintMode::Fix));
        assert_eq!(o.content, "그가 말했다. 그래서 그래서 끝");
        assert!(o.findings[0].fixed);
        let o = run(&"하나둘셋넷".repeat(12), "나", &only_stuck(LintMode::Fix));
        assert_eq!(o.content, "하나둘셋넷하나둘셋넷");
        // 표시만이면 글은 그대로
        let t = "그래서 ".repeat(10);
        assert_eq!(run(&t, "나", &only_stuck(LintMode::Report)).content, t);
    }

    #[test]
    fn a_response_cut_at_the_limit_is_judged_a_little_stricter() {
        let t = "그래서 ".repeat(6);
        assert!(run_with(&t, "나", &only_stuck(LintMode::Report), false).findings.is_empty());
        assert_eq!(run_with(&t, "나", &only_stuck(LintMode::Report), true).findings.len(), 1);
    }

    #[test]
    fn banned_words_use_boundaries_for_latin_and_substrings_for_hangul() {
        let s = |words: &[&str], mode| LintSetting {
            unclosed: LintMode::Off,
            repeat: LintMode::Off,
            stuck: LintMode::Off,
            impersonate: LintMode::Off,
            length: LintMode::Off,
            banned: mode,
            banned_words: words.iter().map(|w| w.to_string()).collect(),
            ..Default::default()
        };
        assert!(
            run("He said hello", "나", &s(&["ai"], LintMode::Report)).findings.is_empty(),
            "said에 걸리지 않음"
        );
        assert_eq!(
            run("The AI replied. ai!", "나", &s(&["ai"], LintMode::Report)).findings[0].text,
            "금지어: ai ×2"
        );
        let o = run(
            "알량한 그의 알량한을 본 사슴",
            "나",
            &s(&["알량한", "헤드라이트 사슴", "알량한"], LintMode::Report),
        );
        assert_eq!(o.findings[0].detail.as_deref(), Some("알량한"), "걸린 단어만, 중복 없이 나열: {:?}", o.findings);
        let o = run("알량한 척하는 사슴", "나", &s(&["알량한", "사슴"], LintMode::Fix));
        assert_eq!(o.content, " 척하는 ");
        assert_eq!(o.findings[0].detail.as_deref(), Some("알량한\u{1f}사슴"));
    }
}
