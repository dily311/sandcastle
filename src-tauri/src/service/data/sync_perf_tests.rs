//! 동기화 대용량 측정 (1-F-3-1). 오래 걸려서 `#[ignore]` — 직접 돌린다:
//!
//! ```text
//! cargo test --release --lib sync_perf -- --ignored --nocapture
//! SYNC_PERF_ROWS=100000 SYNC_PERF_FILES=20 SYNC_PERF_FILE_MB=50 cargo test --release --lib sync_perf -- --ignored --nocapture
//! ```
//!
//! 합성 데이터(캐릭터 행 N개 + 파일 M개×크기)로 묶음 만들기·열기·합치기·파일 해시·폴더 동기화 왕복의 시간과 최대 메모리(프로세스 전체)를 찍는다.
//! 결과는 `docs/성능 측정.md`에 옮긴다.
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use serde_json::Map;

use super::sync_bundle::{self, BundleOptions};
use super::sync_merge::{merge, BundleData};
use super::sync_run;
use super::sync_target::FolderTarget;

fn env(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// 프로세스 물리 메모리(MB)를 50ms마다 재서 최대값을 기록한다
struct PeakMem {
    stop: Arc<AtomicBool>,
    peak: Arc<AtomicUsize>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl PeakMem {
    fn start() -> Self {
        let (stop, peak) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicUsize::new(0)));
        let (s, p) = (stop.clone(), peak.clone());
        let handle = std::thread::spawn(move || {
            while !s.load(Ordering::Relaxed) {
                if let Some(m) = memory_stats::memory_stats() {
                    p.fetch_max(m.physical_mem, Ordering::Relaxed);
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });
        Self {
            stop,
            peak,
            handle: Some(handle),
        }
    }
    /// 지금까지의 최대 (MB)
    fn mb(&self) -> usize {
        self.peak.load(Ordering::Relaxed) / (1024 * 1024)
    }
}

impl Drop for PeakMem {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// 서로 다른 내용의 파일 (같은 해시가 되지 않게 번호를 섞은 의사 난수)
fn write_file(path: &Path, mb: u64, seed: u64) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut chunk = vec![0u8; 1024 * 1024];
    for _ in 0..mb {
        for b in chunk.iter_mut() {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *b = x as u8;
        }
        f.write_all(&chunk).unwrap();
    }
}

fn line(what: &str, t: Instant, peak: &PeakMem, extra: &str) {
    println!("| {what} | {:.2}s | {}MB | {extra} |", t.elapsed().as_secs_f64(), peak.mb());
}

#[tokio::test]
#[ignore = "대용량 측정 — 직접 실행"]
async fn sync_perf_report() {
    let rows = env("SYNC_PERF_ROWS", 10_000);
    let files = env("SYNC_PERF_FILES", 5);
    let file_mb = env("SYNC_PERF_FILE_MB", 20);
    let root = tempfile::tempdir().unwrap();
    let (a_base, b_base) = (root.path().join("a"), root.path().join("b"));
    let a = crate::db::conn::init_pool(a_base.clone()).await.unwrap();
    let b = crate::db::conn::init_pool(b_base.clone()).await.unwrap();
    let (a_id, b_id) = (uuid::Uuid::new_v4().to_string(), uuid::Uuid::new_v4().to_string());

    println!("\n행 {rows}개 · 파일 {files}개 × {file_mb}MB = {}MB", files * file_mb);
    println!("| 단계 | 시간 | 최대 메모리 | 비고 |\n| --- | --- | --- | --- |");

    // 데이터 만들기 (측정 밖)
    sqlx::query("WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < ?) INSERT INTO characters (id, name, description, created_at, updated_at) SELECT 'c' || i, 'name' || i, hex(randomblob(200)), 1, 0 FROM n")
        .bind(rows as i64)
        .execute(&a)
        .await
        .unwrap();
    for i in 0..files {
        write_file(&a_base.join(format!("characters/avatars/f{i}.bin")), file_mb, i + 1);
    }
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let peak = PeakMem::start();
    let base_mb = peak.mb();
    let _ = base_mb;

    let t = Instant::now();
    let list = sync_bundle::list_files(&a_base).unwrap();
    line("파일 해시(목록 만들기)", t, &peak, &format!("{}개", list.len()));
    // 해시 캐시가 있다면 두 번째가 훨씬 빨라야 한다
    let t = Instant::now();
    let _ = sync_bundle::list_files(&a_base).unwrap();
    line("파일 해시 다시", t, &peak, "같은 파일 그대로");

    let out = root.path().join("a.scsync");
    let t = Instant::now();
    sync_bundle::write_bundle(&a, &a_base, &Map::new(), &a_id, &BundleOptions::default(), &out)
        .await
        .unwrap();
    line(
        "묶음 만들기(파일 내용 빼고)",
        t,
        &peak,
        &format!("{}KB", std::fs::metadata(&out).unwrap().len() / 1024),
    );

    let t = Instant::now();
    let local = sync_bundle::schema_version(&b).await.unwrap();
    let mut bundle = sync_bundle::open(&out, local, &b_base).unwrap();
    let data = BundleData::read(&mut bundle, &b).await.unwrap();
    line("묶음 열기·읽기", t, &peak, "");

    let t = Instant::now();
    let rep = merge(&b, &data, &b_id, &mut Map::new(), false).await.unwrap();
    line("합치기(빈 기기에)", t, &peak, &format!("추가 {}", rep.added));

    // 폴더 동기화 왕복: A가 올리고 B가 받는다 (파일 포함)
    let shared = root.path().join("shared");
    std::fs::create_dir_all(&shared).unwrap();
    let folder = FolderTarget::new(&shared).unwrap();
    let c_base = root.path().join("c");
    let c = crate::db::conn::init_pool(c_base.clone()).await.unwrap();
    let c_id = uuid::Uuid::new_v4().to_string();
    let t = Instant::now();
    sync_run::run(&a, &a_base, &mut Map::new(), &a_id, &folder, &|_| {}).await.unwrap();
    line("동기화: 올리는 기기", t, &peak, "묶음 + 파일");
    let t = Instant::now();
    let r = sync_run::run(&c, &c_base, &mut Map::new(), &c_id, &folder, &|_| {}).await.unwrap();
    line("동기화: 받는 기기(처음)", t, &peak, &format!("파일 {}개 받음", r.files_down));
    let t = Instant::now();
    let r = sync_run::run(&a, &a_base, &mut Map::new(), &a_id, &folder, &|_| {}).await.unwrap();
    line("동기화: 바뀐 것 없이 다시", t, &peak, &format!("올린 파일 {}", r.files_up));
    println!("최대 메모리는 프로세스 전체(시험 도구 포함) 기준");
}
