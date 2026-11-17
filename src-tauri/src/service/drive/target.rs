//! 구글 드라이브 앱 전용 폴더를 동기화 저장소로 (CLAUDE.md 2-D-2, ADR-011).
//!
//! - `appDataFolder` 안에만 둔다. 이름은 폴더 대상과 같은 `devices/<id>.scsync`, `blobs/<해시>` (드라이브 이름에 `/`를 그대로 씀)
//! - 목록은 한 번 다 읽어 이름 → id 표를 만든다(동기화 한 번 동안). 같은 이름이 둘이면 먼저 것
//! - 올리기는 resumable 업로드를 조각(기본 8MB)으로 — 큰 파일이 중간에 끊겨도 그 조각부터 다시(세 번까지)
//! - 401이면 access token을 새로 받아 한 번 더
use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;

use super::auth::{self, Client, Endpoints};
use crate::error::{AppError, ErrorCode, Result};
use crate::service::data::sync_target::{RemoteEntry, SyncTarget};

const CHUNK: usize = 8 * 1024 * 1024;

pub struct DriveTarget {
    http: reqwest::Client,
    client: Client,
    ep: Endpoints,
    chunk: usize,
    /// 이름 → (id, 크기)
    index: Mutex<Option<HashMap<String, (String, u64)>>>,
}

#[derive(Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<DriveFile>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct DriveFile {
    id: String,
    name: String,
    #[serde(default)]
    size: Option<String>,
}

fn err(msg: impl Into<String>) -> AppError {
    AppError::ai_provider(ErrorCode::AiProviderConnectionFailed, msg, true)
}

impl DriveTarget {
    pub fn new(http: reqwest::Client, client: Client, ep: Endpoints) -> Self {
        Self {
            http,
            client,
            ep,
            chunk: CHUNK,
            index: Mutex::new(None),
        }
    }

    #[cfg(test)]
    fn with_chunk(mut self, chunk: usize) -> Self {
        self.chunk = chunk;
        self
    }

    /// 인증을 붙여 보내고, 401이면 토큰을 새로 받아 한 번 더
    async fn send(&self, build: impl Fn(&reqwest::Client, &str) -> reqwest::RequestBuilder) -> Result<reqwest::Response> {
        for attempt in 0..2 {
            let token = auth::access_token(&self.http, &self.client, &self.ep).await?;
            let res = build(&self.http, &token)
                .send()
                .await
                .map_err(|e| err(format!("구글 드라이브에 연결하지 못함: {e}")))?;
            if res.status() == reqwest::StatusCode::UNAUTHORIZED && attempt == 0 {
                auth::forget_access_token();
                continue;
            }
            return Ok(res);
        }
        Err(err("구글 드라이브 인증 실패"))
    }

    async fn ensure_index(&self) -> Result<()> {
        let mut idx = self.index.lock().await;
        if idx.is_some() {
            return Ok(());
        }
        let mut map: HashMap<String, (String, u64)> = HashMap::new();
        let mut page: Option<String> = None;
        loop {
            let url = format!("{}/files", self.ep.api);
            let p = page.clone();
            let res = self
                .send(|c, t| {
                    let mut q = vec![
                        ("spaces", "appDataFolder".to_string()),
                        ("fields", "nextPageToken,files(id,name,size)".to_string()),
                        ("pageSize", "1000".to_string()),
                    ];
                    if let Some(p) = &p {
                        q.push(("pageToken", p.clone()));
                    }
                    c.get(&url).bearer_auth(t).query(&q)
                })
                .await?;
            if !res.status().is_success() {
                return Err(err(format!("드라이브 목록을 읽지 못함 (HTTP {})", res.status())));
            }
            let list: FileList = res.json().await.map_err(|e| err(format!("드라이브 응답을 읽지 못함: {e}")))?;
            for f in list.files {
                let size = f.size.and_then(|s| s.parse().ok()).unwrap_or(0);
                map.entry(f.name).or_insert((f.id, size));
            }
            match list.next_page_token {
                Some(p) => page = Some(p),
                None => break,
            }
        }
        *idx = Some(map);
        Ok(())
    }

    async fn id_of(&self, name: &str) -> Result<Option<String>> {
        self.ensure_index().await?;
        Ok(self.index.lock().await.as_ref().and_then(|m| m.get(name).map(|v| v.0.clone())))
    }

    /// 연결한 계정의 이메일 (화면 표시용)
    pub async fn account_email(&self) -> Result<Option<String>> {
        let url = format!("{}/about", self.ep.api);
        let res = self.send(|c, t| c.get(&url).bearer_auth(t).query(&[("fields", "user(emailAddress)")])).await?;
        if !res.status().is_success() {
            return Ok(None);
        }
        let v: serde_json::Value = res.json().await.unwrap_or_default();
        Ok(v["user"]["emailAddress"].as_str().map(String::from))
    }

    /// 드라이브의 앱 전용 폴더를 비운다 (연결 끊기에서 사용자가 고를 때만)
    pub async fn delete_all(&self) -> Result<u32> {
        self.ensure_index().await?;
        let names: Vec<String> = self.index.lock().await.as_ref().map(|m| m.keys().cloned().collect()).unwrap_or_default();
        for n in &names {
            self.delete(n).await?;
        }
        Ok(names.len() as u32)
    }
}

impl SyncTarget for DriveTarget {
    fn kind(&self) -> &'static str {
        "drive"
    }

    async fn list(&self, prefix: &str) -> Result<Vec<RemoteEntry>> {
        self.ensure_index().await?;
        let idx = self.index.lock().await;
        let mut out: Vec<RemoteEntry> = idx
            .as_ref()
            .map(|m| {
                m.iter()
                    .filter(|(n, _)| n.starts_with(prefix) && !n[prefix.len()..].contains('/'))
                    .map(|(n, (_, size))| RemoteEntry { name: n.clone(), size: *size })
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    async fn get(&self, name: &str, dest: &Path) -> Result<()> {
        let id = self.id_of(name).await?.ok_or_else(|| AppError::not_found(format!("드라이브에 없음: {name}")))?;
        let url = format!("{}/files/{id}", self.ep.api);
        let mut res = self.send(|c, t| c.get(&url).bearer_auth(t).query(&[("alt", "media")])).await?;
        if !res.status().is_success() {
            return Err(err(format!("드라이브에서 받지 못함 ({name}, HTTP {})", res.status())));
        }
        if let Some(p) = dest.parent() {
            std::fs::create_dir_all(p)?;
        }
        let mut f = tokio::fs::File::create(dest).await?;
        while let Some(chunk) = res.chunk().await.map_err(|e| err(format!("드라이브에서 받다가 끊김: {e}")))? {
            f.write_all(&chunk).await?;
        }
        f.flush().await?;
        Ok(())
    }

    async fn put(&self, name: &str, src: &Path) -> Result<()> {
        let total = std::fs::metadata(src)?.len();
        let existing = self.id_of(name).await?;
        // 1) 업로드 자리 만들기
        let meta = serde_json::json!({ "name": name, "parents": ["appDataFolder"] });
        let res = match &existing {
            Some(id) => {
                let url = format!("{}/files/{id}", self.ep.upload);
                self.send(|c, t| c.patch(&url).bearer_auth(t).query(&[("uploadType", "resumable")]).json(&serde_json::json!({})))
                    .await?
            }
            None => {
                let url = format!("{}/files", self.ep.upload);
                self.send(|c, t| c.post(&url).bearer_auth(t).query(&[("uploadType", "resumable")]).json(&meta))
                    .await?
            }
        };
        if !res.status().is_success() {
            return Err(err(format!("드라이브에 올릴 자리를 만들지 못함 ({name}, HTTP {})", res.status())));
        }
        let session = res
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| err("드라이브가 업로드 주소를 주지 않음"))?
            .to_string();

        // 2) 조각으로 올리기 (끊기면 그 조각부터 세 번까지)
        let mut file = tokio::fs::File::open(src).await?;
        let mut offset: u64 = 0;
        let mut buf = vec![0u8; self.chunk];
        let mut done: Option<DriveFile> = None;
        while done.is_none() {
            file.seek(std::io::SeekFrom::Start(offset)).await?;
            let mut n = 0;
            while n < buf.len() {
                let r = file.read(&mut buf[n..]).await?;
                if r == 0 {
                    break;
                }
                n += r;
            }
            let end = offset + n as u64;
            let range = if total == 0 {
                "bytes */0".to_string()
            } else {
                format!("bytes {offset}-{}/{total}", end.saturating_sub(1))
            };
            let body = buf[..n].to_vec();
            let mut last_err = None;
            let mut res = None;
            for _ in 0..3 {
                // 업로드 세션 주소에는 인증이 따로 필요 없다
                match self
                    .http
                    .put(&session)
                    .header(reqwest::header::CONTENT_RANGE, &range)
                    .body(body.clone())
                    .send()
                    .await
                {
                    Ok(r) => {
                        res = Some(r);
                        break;
                    }
                    Err(e) => last_err = Some(e),
                }
            }
            let res = res.ok_or_else(|| {
                err(format!(
                    "드라이브에 올리다가 끊김 ({name}): {}",
                    last_err.map(|e| e.to_string()).unwrap_or_default()
                ))
            })?;
            match res.status().as_u16() {
                200 | 201 => done = Some(res.json().await.map_err(|e| err(format!("드라이브 응답을 읽지 못함: {e}")))?),
                308 => {
                    // 받은 곳까지: "Range: bytes=0-123"
                    offset = res
                        .headers()
                        .get(reqwest::header::RANGE)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|r| r.rsplit('-').next())
                        .and_then(|e| e.parse::<u64>().ok())
                        .map_or(0, |e| e + 1);
                }
                s => return Err(err(format!("드라이브에 올리지 못함 ({name}, HTTP {s})"))),
            }
        }
        let f = done.unwrap_or(DriveFile {
            id: String::new(),
            name: name.into(),
            size: None,
        });
        if let Some(m) = self.index.lock().await.as_mut() {
            m.insert(name.to_string(), (f.id, total));
        }
        Ok(())
    }

    async fn delete(&self, name: &str) -> Result<()> {
        let Some(id) = self.id_of(name).await? else { return Ok(()) };
        let url = format!("{}/files/{id}", self.ep.api);
        let res = self.send(|c, t| c.delete(&url).bearer_auth(t)).await?;
        if !res.status().is_success() && res.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(err(format!("드라이브에서 지우지 못함 ({name}, HTTP {})", res.status())));
        }
        if let Some(m) = self.index.lock().await.as_mut() {
            m.remove(name);
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod fake {
    //! 테스트용 가짜 드라이브 (HTTP/1.1을 손으로 읽는 작은 서버). 실제 구글에 요청하지 않는다
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[derive(Default)]
    pub struct State {
        pub files: HashMap<String, (String, Vec<u8>)>,                // id → (name, bytes)
        sessions: HashMap<String, (Option<String>, String, Vec<u8>)>, // sid → (기존 id, name, 받은 bytes)
        next: u32,
        pub token_calls: u32,
        pub expire_next: bool,
        pub puts: u32,
    }

    pub async fn start() -> (String, Arc<Mutex<State>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(State::default()));
        let st = state.clone();
        let b = base.clone();
        tokio::spawn(async move {
            loop {
                let Ok((sock, _)) = listener.accept().await else { break };
                let (st, b) = (st.clone(), b.clone());
                tokio::spawn(async move { serve(sock, st, b).await });
            }
        });
        (base, state)
    }

    async fn serve(mut sock: tokio::net::TcpStream, st: Arc<Mutex<State>>, base: String) {
        let mut buf: Vec<u8> = vec![];
        loop {
            // 머리
            let head_end = loop {
                if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    break p + 4;
                }
                let mut tmp = [0u8; 65536];
                let n = match sock.read(&mut tmp).await {
                    Ok(0) | Err(_) => return,
                    Ok(n) => n,
                };
                buf.extend_from_slice(&tmp[..n]);
            };
            let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
            let mut lines = head.lines();
            let first: Vec<&str> = lines.next().unwrap_or("").split_whitespace().collect();
            let (method, target) = (first[0].to_string(), first[1].to_string());
            let headers: HashMap<String, String> = lines
                .filter_map(|l| l.split_once(':'))
                .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
                .collect();
            let len: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
            while buf.len() < head_end + len {
                let mut tmp = [0u8; 65536];
                let n = match sock.read(&mut tmp).await {
                    Ok(0) | Err(_) => return,
                    Ok(n) => n,
                };
                buf.extend_from_slice(&tmp[..n]);
            }
            let body = buf[head_end..head_end + len].to_vec();
            buf.drain(..head_end + len);
            let (status, extra, out) = handle(&st, &base, &method, &target, &headers, &body);
            let resp = format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\n{extra}\r\n", out.len());
            if sock.write_all(resp.as_bytes()).await.is_err() || sock.write_all(&out).await.is_err() {
                return;
            }
        }
    }

    fn handle(st: &Mutex<State>, base: &str, method: &str, target: &str, h: &HashMap<String, String>, body: &[u8]) -> (u16, String, Vec<u8>) {
        let mut s = st.lock().unwrap();
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        let json = |v: serde_json::Value| (200u16, "Content-Type: application/json\r\n".to_string(), v.to_string().into_bytes());
        if path == "/token" {
            s.token_calls += 1;
            return json(serde_json::json!({ "access_token": format!("at{}", s.token_calls), "expires_in": 3600 }));
        }
        if path != "/token" && !path.starts_with("/session/") {
            let auth = h.get("authorization").cloned().unwrap_or_default();
            if s.expire_next || !auth.starts_with("Bearer at") {
                s.expire_next = false;
                return (401, String::new(), vec![]);
            }
        }
        match (method, path) {
            ("GET", "/api/files") => {
                assert!(query.contains("spaces=appDataFolder"), "앱 전용 폴더만");
                let files: Vec<_> = s
                    .files
                    .iter()
                    .map(|(id, (name, b))| serde_json::json!({ "id": id, "name": name, "size": b.len().to_string() }))
                    .collect();
                json(serde_json::json!({ "files": files }))
            }
            ("GET", p) if p.starts_with("/api/files/") => {
                let id = &p["/api/files/".len()..];
                match s.files.get(id) {
                    Some((_, b)) => (200, String::new(), b.clone()),
                    None => (404, String::new(), vec![]),
                }
            }
            ("DELETE", p) if p.starts_with("/api/files/") => {
                s.files.remove(&p["/api/files/".len()..]);
                (204, String::new(), vec![])
            }
            ("POST", "/upload/files") | ("PATCH", _) => {
                assert!(query.contains("uploadType=resumable"));
                let existing = (method == "PATCH").then(|| path["/upload/files/".len()..].to_string());
                let name = if let Some(id) = &existing {
                    s.files.get(id).map(|f| f.0.clone()).unwrap_or_default()
                } else {
                    let m: serde_json::Value = serde_json::from_slice(body).unwrap();
                    assert_eq!(m["parents"][0], "appDataFolder");
                    m["name"].as_str().unwrap().to_string()
                };
                s.next += 1;
                let sid = format!("s{}", s.next);
                s.sessions.insert(sid.clone(), (existing, name, vec![]));
                (200, format!("Location: {base}/session/{sid}\r\n"), vec![])
            }
            ("PUT", p) if p.starts_with("/session/") => {
                s.puts += 1;
                let sid = p["/session/".len()..].to_string();
                let range = h.get("content-range").cloned().unwrap_or_default();
                let total: usize = range.rsplit('/').next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let sess = s.sessions.get_mut(&sid).unwrap();
                sess.2.extend_from_slice(body);
                if sess.2.len() < total {
                    return (308, format!("Range: bytes=0-{}\r\n", sess.2.len() - 1), vec![]);
                }
                let (existing, name, bytes) = s.sessions.remove(&sid).unwrap();
                let id = existing.unwrap_or_else(|| {
                    s.next += 1;
                    format!("f{}", s.next)
                });
                s.files.insert(id.clone(), (name.clone(), bytes));
                json(serde_json::json!({ "id": id, "name": name }))
            }
            _ => (404, String::new(), vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn endpoints(base: &str) -> Endpoints {
        Endpoints {
            auth: format!("{base}/auth"),
            token: format!("{base}/token"),
            revoke: format!("{base}/revoke"),
            api: format!("{base}/api"),
            upload: format!("{base}/upload"),
        }
    }

    #[tokio::test]
    async fn drive_target_round_trip_with_chunks_and_token_refresh() {
        let (base, state) = fake::start().await;
        auth::set_refresh_for_test("rt-1");
        let client = Client {
            id: "cid".into(),
            secret: String::new(),
        };
        let t = DriveTarget::new(reqwest::Client::new(), client.clone(), endpoints(&base)).with_chunk(4);
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.bin");
        std::fs::write(&src, b"hello drive!").unwrap(); // 12바이트 → 4바이트 조각 3개

        t.put("devices/d1.scsync", &src).await.unwrap();
        t.put("blobs/abc", &src).await.unwrap();
        assert_eq!(state.lock().unwrap().puts, 6, "조각으로 나눠 올림");
        assert_eq!(
            t.list("devices/").await.unwrap(),
            vec![RemoteEntry {
                name: "devices/d1.scsync".into(),
                size: 12
            }]
        );

        // 새 대상(새 목록)으로 읽기
        let t2 = DriveTarget::new(reqwest::Client::new(), client.clone(), endpoints(&base));
        let out = dir.path().join("got.bin");
        t2.get("devices/d1.scsync", &out).await.unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"hello drive!");

        // 같은 이름에 다시 올리면 바꿔 넣는다 (파일이 늘지 않음)
        std::fs::write(&src, b"v2").unwrap();
        t2.put("devices/d1.scsync", &src).await.unwrap();
        assert_eq!(state.lock().unwrap().files.len(), 2);
        let t3 = DriveTarget::new(reqwest::Client::new(), client.clone(), endpoints(&base));
        t3.get("devices/d1.scsync", &out).await.unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"v2");

        // 토큰이 만료되면(401) 새로 받아 한 번 더
        let before = state.lock().unwrap().token_calls;
        state.lock().unwrap().expire_next = true;
        t3.delete("blobs/abc").await.unwrap();
        assert_eq!(state.lock().unwrap().token_calls, before + 1);
        assert!(t3.list("blobs/").await.unwrap().is_empty());

        assert_eq!(t3.delete_all().await.unwrap(), 1);
        assert!(state.lock().unwrap().files.is_empty());
    }

    /// 폴더 대상과 같은 sync_run이 드라이브로도 돈다 (기기 둘)
    #[tokio::test]
    async fn sync_run_over_fake_drive() {
        use crate::service::data::sync_run;
        let (base, _state) = fake::start().await;
        auth::set_refresh_for_test("rt-1");
        let client = Client {
            id: "cid".into(),
            secret: String::new(),
        };
        let root = tempfile::tempdir().unwrap();
        let (a_base, b_base) = (root.path().join("a"), root.path().join("b"));
        let a = crate::db::conn::init_pool(a_base.clone()).await.unwrap();
        let b = crate::db::conn::init_pool(b_base.clone()).await.unwrap();
        let img = a_base.join("generated/1.png");
        std::fs::create_dir_all(img.parent().unwrap()).unwrap();
        std::fs::write(&img, b"png").unwrap();
        sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c1', 'Aria', 1, 0)")
            .execute(&a)
            .await
            .unwrap();
        let (ida, idb) = (uuid::Uuid::new_v4().to_string(), uuid::Uuid::new_v4().to_string());

        let t = DriveTarget::new(reqwest::Client::new(), client.clone(), endpoints(&base));
        let r = sync_run::run(&a, &a_base, &mut serde_json::Map::new(), &ida, &t, &|_| {}).await.unwrap();
        assert_eq!(r.files_up, 1);
        let t = DriveTarget::new(reqwest::Client::new(), client.clone(), endpoints(&base));
        let r = sync_run::run(&b, &b_base, &mut serde_json::Map::new(), &idb, &t, &|_| {}).await.unwrap();
        assert_eq!((r.merged.len(), r.files_down), (1, 1));
        let n: String = sqlx::query_scalar("SELECT name FROM characters WHERE id = 'c1'").fetch_one(&b).await.unwrap();
        assert_eq!(n, "Aria");
        assert_eq!(std::fs::read(b_base.join("generated/1.png")).unwrap(), b"png");
    }
}
