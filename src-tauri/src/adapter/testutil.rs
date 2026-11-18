//! 어댑터 시험용 가짜 공급자 서버. 실제 공급자에 요청하지 않는다.
//!
//! 정해 둔 답을 차례로 돌려주고, 받은 요청(경로·머리·본문)을 모아 둔다. SSE 답은 청크 단위로 보내서
//! (글자 중간에서 자를 수도 있다) 스트리밍 경계 처리를 시험할 수 있다.
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::models::ai_service::ApiProfiles;

#[derive(Debug, Clone)]
pub struct Captured {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Captured {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
}

pub enum Reply {
    Json(u16, Value),
    /// 청크마다 따로 보낸다 (바이트 단위라 UTF-8 글자 중간에서 잘라도 된다)
    Sse(Vec<Vec<u8>>),
}

impl Reply {
    pub fn ok(v: Value) -> Self {
        Self::Json(200, v)
    }
    /// SSE 줄들을 청크로 나눈다 (한 줄이 한 청크)
    pub fn sse_lines(lines: &[&str]) -> Self {
        Self::Sse(lines.iter().map(|l| format!("{l}\n\n").into_bytes()).collect())
    }
}

pub struct FakeProvider {
    pub base: String,
    pub seen: Arc<Mutex<Vec<Captured>>>,
}

impl FakeProvider {
    pub fn requests(&self) -> Vec<Captured> {
        self.seen.lock().unwrap().clone()
    }
    pub fn last(&self) -> Captured {
        self.requests().pop().expect("요청이 없음")
    }
}

pub async fn serve(replies: Vec<Reply>) -> FakeProvider {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(vec![]));
    let queue = Arc::new(Mutex::new(replies.into_iter().collect::<VecDeque<_>>()));
    let (s, q) = (seen.clone(), queue);
    tokio::spawn(async move {
        loop {
            let Ok((sock, _)) = listener.accept().await else { break };
            let (s, q) = (s.clone(), q.clone());
            tokio::spawn(async move { connection(sock, s, q).await });
        }
    });
    FakeProvider { base, seen }
}

async fn connection(mut sock: tokio::net::TcpStream, seen: Arc<Mutex<Vec<Captured>>>, queue: Arc<Mutex<VecDeque<Reply>>>) {
    let mut buf: Vec<u8> = vec![];
    loop {
        let head_end = loop {
            if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break p + 4;
            }
            let mut tmp = [0u8; 65536];
            match sock.read(&mut tmp).await {
                Ok(0) | Err(_) => return,
                Ok(n) => buf.extend_from_slice(&tmp[..n]),
            }
        };
        let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
        let mut lines = head.lines();
        let first: Vec<&str> = lines.next().unwrap_or("").split_whitespace().collect();
        let headers: HashMap<String, String> = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
            .collect();
        let len: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
        while buf.len() < head_end + len {
            let mut tmp = [0u8; 65536];
            match sock.read(&mut tmp).await {
                Ok(0) | Err(_) => return,
                Ok(n) => buf.extend_from_slice(&tmp[..n]),
            }
        }
        let body = buf[head_end..head_end + len].to_vec();
        buf.drain(..head_end + len);
        seen.lock().unwrap().push(Captured {
            method: first.first().unwrap_or(&"").to_string(),
            path: first.get(1).unwrap_or(&"").to_string(),
            headers,
            body,
        });
        let reply = queue.lock().unwrap().pop_front();
        let ok = match reply {
            Some(Reply::Json(status, v)) => {
                let body = v.to_string();
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                );
                sock.write_all(head.as_bytes()).await.is_ok() && sock.write_all(body.as_bytes()).await.is_ok()
            }
            Some(Reply::Sse(chunks)) => {
                let mut ok = sock
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n")
                    .await
                    .is_ok();
                for c in chunks {
                    ok = ok
                        && sock.write_all(format!("{:x}\r\n", c.len()).as_bytes()).await.is_ok()
                        && sock.write_all(&c).await.is_ok()
                        && sock.write_all(b"\r\n").await.is_ok();
                    let _ = sock.flush().await;
                    tokio::time::sleep(Duration::from_millis(4)).await; // 청크가 따로 읽히게
                }
                ok && sock.write_all(b"0\r\n\r\n").await.is_ok()
            }
            None => sock.write_all(b"HTTP/1.1 500 X\r\nContent-Length: 0\r\n\r\n").await.is_ok(),
        };
        if !ok {
            return;
        }
    }
}

/// 가짜 서버를 가리키는 API 프로필 (키는 "test-key")
pub fn profile(base: &str, format_type: &str, model: &str) -> (reqwest::Client, ApiProfiles) {
    (
        reqwest::Client::new(),
        ApiProfiles {
            id: "p1".into(),
            name: "시험".into(),
            provider: format_type.into(),
            format_type: format_type.into(),
            base_url: base.into(),
            model: model.into(),
            created_at: 0,
            updated_at: 0,
        },
    )
}
