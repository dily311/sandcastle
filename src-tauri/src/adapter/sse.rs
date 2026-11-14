//! Server-Sent Events 읽기 도우미.
//!
//! 바이트 청크 경계가 줄/UTF-8 문자 중간에 걸려도 안전하도록 줄 단위로 모아서 해석한다.
use futures::StreamExt;
use reqwest::Response;

use crate::{adapter::net_err, error::Result};

/// SSE 한 건
pub(crate) struct SseEvent<'a> {
    /// `event:` 줄 (없으면 None)
    pub event: Option<&'a str>,
    /// `data:` 줄 내용
    pub data: &'a str,
}

/// 응답 본문을 SSE로 읽으며 `data:` 줄마다 콜백을 부른다. 콜백이 `Ok(false)`를 돌려주면 멈춘다.
pub(crate) async fn read_sse<F>(resp: Response, mut on_event: F) -> Result<()>
where
    F: FnMut(SseEvent<'_>) -> Result<bool>,
{
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    let mut event: Option<String> = None;

    while let Some(chunk) = stream.next().await {
        buf.extend_from_slice(&chunk.map_err(net_err)?);
        while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line_bytes: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes);
            if !handle_line(line.trim_end_matches(['\r', '\n']), &mut event, &mut on_event)? {
                return Ok(());
            }
        }
    }
    // 마지막 줄이 개행 없이 끝난 경우
    if !buf.is_empty() {
        let line = String::from_utf8_lossy(&buf).to_string();
        handle_line(line.trim_end_matches('\r'), &mut event, &mut on_event)?;
    }
    Ok(())
}

fn handle_line<F>(line: &str, event: &mut Option<String>, on_event: &mut F) -> Result<bool>
where
    F: FnMut(SseEvent<'_>) -> Result<bool>,
{
    if line.is_empty() {
        // 빈 줄 = 이벤트 경계
        *event = None;
        return Ok(true);
    }
    if line.starts_with(':') {
        return Ok(true); // 주석/keep-alive
    }
    if let Some(name) = line.strip_prefix("event:") {
        *event = Some(name.trim().to_string());
        return Ok(true);
    }
    if let Some(data) = line.strip_prefix("data:") {
        let data = data.strip_prefix(' ').unwrap_or(data);
        return on_event(SseEvent {
            event: event.as_deref(),
            data,
        });
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_event_and_data_lines() {
        let mut event = None;
        let mut seen = vec![];
        let lines = [
            ": ping",
            "event: content_block_delta",
            "data: {\"a\":1}",
            "",
            "data:[DONE]",
        ];
        for l in lines {
            handle_line(l, &mut event, &mut |e: SseEvent<'_>| {
                seen.push((e.event.map(str::to_string), e.data.to_string()));
                Ok(true)
            })
            .unwrap();
        }
        assert_eq!(
            seen,
            vec![
                (Some("content_block_delta".into()), "{\"a\":1}".into()),
                (None, "[DONE]".into()),
            ]
        );
    }
}
