//! 커뮤니티 플러그인 설치 테스트 (CLAUDE.md 6-D-1)
use std::{
    io::Write,
    path::{Path, PathBuf},
};

use serde_json::json;
use zip::{write::SimpleFileOptions, ZipWriter};

use super::*;
use crate::{db, db::conn::DbPool, service::data::trash};

async fn app_dir() -> (tempfile::TempDir, PathBuf, DbPool) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("appdata");
    let db = db::conn::init_pool(base.clone()).await.unwrap();
    (dir, base, db)
}

fn manifest(id: &str, version: &str, perms: &[&str]) -> String {
    json!({ "id": id, "name": "Word Count", "version": version, "apiVersion": 1, "author": "me", "permissions": perms }).to_string()
}

/// 폴더로 된 플러그인
fn folder(root: &Path, files: &[(&str, &str)]) -> PathBuf {
    let dir = root.join(format!("src-{}", uuid::Uuid::new_v4().simple()));
    for (rel, body) in files {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    dir
}

fn zip_of(root: &Path, files: &[(&str, &[u8])]) -> PathBuf {
    let path = root.join(format!("p-{}.zip", uuid::Uuid::new_v4().simple()));
    let mut zip = ZipWriter::new(std::fs::File::create(&path).unwrap());
    for (name, body) in files {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(body).unwrap();
    }
    zip.finish().unwrap();
    path
}

async fn install_from(db: &DbPool, base: &Path, src: &Path) -> Result<UserPlugin> {
    let p = preview(db, base, src).await?;
    install(db, base, src, &p.hash).await
}

#[tokio::test]
async fn installs_from_folder_and_zip_disabled_by_default() {
    let (d, base, db) = app_dir().await;
    let src = folder(
        d.path(),
        &[
            ("manifest.json", &manifest("word-count", "1.0.0", &["ui.panel"])),
            ("main.js", "1"),
            ("panel/index.html", "<p>"),
        ],
    );
    let p = install_from(&db, &base, &src).await.unwrap();
    assert_eq!(p.id, "word-count");
    assert!(!p.enabled, "새 플러그인은 꺼진 채로");
    assert!(p.granted.is_empty());
    assert!(base.join("plugins/word-count/panel/index.html").is_file());

    // zip: 폴더 하나로 감싼 것도 (my-plugin/manifest.json), 숨김·macOS 군더더기는 건너뜀
    let z = zip_of(
        d.path(),
        &[
            ("lint/manifest.json", manifest("lint", "0.1.0", &[]).as_bytes()),
            ("lint/main.js", b"2"),
            ("lint/.git/config", b"x"),
            ("__MACOSX/lint/._main.js", b"x"),
        ],
    );
    install_from(&db, &base, &z).await.unwrap();
    assert!(base.join("plugins/lint/main.js").is_file());
    assert!(!base.join("plugins/lint/.git").exists());

    let all = list(&db, &base).await.unwrap();
    assert_eq!(all.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["lint", "word-count"]);
    // 설치 도중의 임시 폴더가 남지 않음
    let leftovers = std::fs::read_dir(base.join("plugins"))
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with('.'))
        .count();
    assert_eq!(leftovers, 0);
}

#[tokio::test]
async fn rejects_bad_manifests() {
    let (d, base, db) = app_dir().await;
    let cases: Vec<(&str, String)> = vec![
        ("id", json!({ "id": "Bad_Id", "name": "x", "version": "1", "apiVersion": 1 }).to_string()),
        ("권한", manifest("p1", "1", &["network"])),
        ("권한", manifest("p2", "1", &["data.read:secrets"])),
        ("apiVersion", json!({ "id": "p3", "name": "x", "version": "1", "apiVersion": 99 }).to_string()),
        ("apiVersion", json!({ "id": "p4", "name": "x", "version": "1" }).to_string()),
        (
            "앱",
            json!({ "id": "p5", "name": "x", "version": "1", "apiVersion": 1, "minAppVersion": "999.0.0" }).to_string(),
        ),
        (
            "main",
            json!({ "id": "p6", "name": "x", "version": "1", "apiVersion": 1, "main": "../x.js" }).to_string(),
        ),
        (
            "settings",
            json!({ "id": "p7", "name": "x", "version": "1", "apiVersion": 1, "settings": [{ "key": "a", "type": "select", "label": "A" }] }).to_string(),
        ),
        ("읽을 수 없음", "{".to_string()),
    ];
    for (want, m) in cases {
        let src = folder(d.path(), &[("manifest.json", &m), ("main.js", "1")]);
        let err = preview(&db, &base, &src).await.expect_err(&m).to_string();
        assert!(err.contains(want), "{m}: {err}");
    }
    // main 파일이 없음
    let src = folder(d.path(), &[("manifest.json", &manifest("p8", "1", &[]))]);
    assert!(preview(&db, &base, &src).await.unwrap_err().to_string().contains("main"));
    // manifest 없음
    let src = folder(d.path(), &[("main.js", "1")]);
    assert!(preview(&db, &base, &src).await.unwrap_err().to_string().contains("manifest"));
    assert!(!base.join("plugins").exists(), "거절하면 아무것도 쓰지 않음");
}

#[tokio::test]
async fn rejects_unsafe_packages() {
    let (d, base, db) = app_dir().await;
    let m = manifest("evil", "1", &[]);
    let bad: Vec<(&str, Vec<(&str, &[u8])>)> = vec![
        ("경로", vec![("manifest.json", m.as_bytes()), ("main.js", b"1"), ("../escape.js", b"x")]),
        ("경로", vec![("manifest.json", m.as_bytes()), ("main.js", b"1"), ("/abs.js", b"x")]),
        ("경로", vec![("manifest.json", m.as_bytes()), ("main.js", b"1"), ("a\\b.js", b"x")]),
        ("종류", vec![("manifest.json", m.as_bytes()), ("main.js", b"1"), ("tool.exe", b"MZ")]),
        ("종류", vec![("manifest.json", m.as_bytes()), ("main.js", b"1"), ("lib.dll", b"MZ")]),
    ];
    for (want, files) in bad {
        let z = zip_of(d.path(), &files);
        let err = preview(&db, &base, &z)
            .await
            .map(|p| format!("{:?}", p.files))
            .expect_err(&format!("{:?}", files.iter().map(|f| f.0).collect::<Vec<_>>()))
            .to_string();
        assert!(err.contains(want), "{err}");
    }
    // 크기 상한
    let big = vec![b'a'; package::MAX_TOTAL_BYTES as usize + 1];
    let z = zip_of(d.path(), &[("manifest.json", m.as_bytes()), ("main.js", &big)]);
    assert!(preview(&db, &base, &z).await.unwrap_err().to_string().contains("너무 큼"));
    // zip이 아닌 파일
    let not_zip = d.path().join("x.zip");
    std::fs::write(&not_zip, b"nope").unwrap();
    assert!(preview(&db, &base, &not_zip).await.is_err());
    assert!(!base.join("plugins").exists());
}

#[tokio::test]
async fn install_checks_preview_hash() {
    let (d, base, db) = app_dir().await;
    let src = folder(d.path(), &[("manifest.json", &manifest("hash", "1", &[])), ("main.js", "1")]);
    let p = preview(&db, &base, &src).await.unwrap();
    assert!(!base.join("plugins").exists(), "미리보기는 쓰지 않음");
    std::fs::write(src.join("main.js"), "changed").unwrap();
    let err = install(&db, &base, &src, &p.hash).await.unwrap_err().to_string();
    assert!(err.contains("바뀌었음"), "{err}");
}

#[tokio::test]
async fn enabling_needs_full_consent_and_new_permissions_disable() {
    let (d, base, db) = app_dir().await;
    let v1 = folder(
        d.path(),
        &[("manifest.json", &manifest("pp", "1.0.0", &["ui.panel", "storage"])), ("main.js", "1")],
    );
    install_from(&db, &base, &v1).await.unwrap();

    // 일부 권한만으로는 켜지지 않음
    assert!(set_enabled(&db, &base, "pp", true, vec!["ui.panel".into()]).await.is_err());
    let on = set_enabled(&db, &base, "pp", true, vec!["storage".into(), "ui.panel".into()]).await.unwrap();
    assert!(on.enabled);
    assert_eq!(on.granted, ["storage", "ui.panel"]);

    // 같은 권한의 새 판 → 켜진 채
    let v2 = folder(
        d.path(),
        &[("manifest.json", &manifest("pp", "1.1.0", &["storage", "ui.panel"])), ("main.js", "2")],
    );
    let pv = preview(&db, &base, &v2).await.unwrap();
    assert_eq!(pv.existing.as_ref().map(|e| e.version.as_str()), Some("1.0.0"));
    assert!(pv.added_permissions.is_empty());
    assert!(!pv.downgrade);
    assert!(install(&db, &base, &v2, &pv.hash).await.unwrap().enabled);
    assert_eq!(std::fs::read_to_string(base.join("plugins/pp/main.js")).unwrap(), "2");

    // 권한이 늘어난 새 판 → 꺼짐 (다시 동의)
    let v3 = folder(
        d.path(),
        &[("manifest.json", &manifest("pp", "2.0.0", &["storage", "ui.panel", "ai"])), ("main.js", "3")],
    );
    let pv = preview(&db, &base, &v3).await.unwrap();
    assert_eq!(pv.added_permissions, ["ai"]);
    let p = install(&db, &base, &v3, &pv.hash).await.unwrap();
    assert!(!p.enabled, "권한이 늘면 꺼져야 함");

    // 낮은 판은 미리보기에 표시
    let pv = preview(&db, &base, &v1).await.unwrap();
    assert!(pv.downgrade);
}

#[tokio::test]
async fn folder_without_state_row_is_listed_disabled() {
    // 예전 백업을 되살렸거나 손으로 넣은 폴더 — 표에 행이 없다
    let (_d, base, db) = app_dir().await;
    let dir = base.join("plugins/manual");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("manifest.json"), manifest("manual", "1", &[])).unwrap();
    std::fs::write(dir.join("main.js"), "1").unwrap();
    let broken = base.join("plugins/broken");
    std::fs::create_dir_all(&broken).unwrap();
    std::fs::write(broken.join("manifest.json"), manifest("other-id", "1", &[])).unwrap();

    let all = list(&db, &base).await.unwrap();
    let manual = all.iter().find(|p| p.id == "manual").unwrap();
    assert!(!manual.enabled && manual.manifest.is_some());
    let broken = all.iter().find(|p| p.id == "broken").unwrap();
    assert!(broken.manifest.is_none());
    assert!(broken.error.as_deref().unwrap().contains("다름"));
    assert!(
        set_enabled(&db, &base, "broken", true, vec![]).await.is_err(),
        "읽지 못한 플러그인은 켤 수 없음"
    );
}

#[tokio::test]
async fn uninstall_goes_to_trash_and_restores_disabled() {
    let (d, base, db) = app_dir().await;
    let src = folder(d.path(), &[("manifest.json", &manifest("tr", "1", &["storage"])), ("main.js", "1")]);
    install_from(&db, &base, &src).await.unwrap();
    set_enabled(&db, &base, "tr", true, vec!["storage".into()]).await.unwrap();

    let item = uninstall(&db, &base, "tr").await.unwrap();
    assert_eq!(item.label, "Word Count");
    assert_eq!(item.files, 1);
    assert!(!base.join("plugins/tr").exists());
    assert!(list(&db, &base).await.unwrap().is_empty());

    trash::restore(&db, &base, &item.id).await.unwrap();
    let back = get(&db, &base, "tr").await.unwrap();
    assert!(!back.enabled, "되살린 플러그인은 꺼진 채");
    assert!(base.join("plugins/tr/main.js").is_file());

    // 폴더만 있는(행 없는) 플러그인도 지울 수 있다
    let dir = base.join("plugins/loose");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("manifest.json"), "{").unwrap();
    uninstall(&db, &base, "loose").await.unwrap();
    assert!(!dir.exists());
}

#[test]
fn plugins_are_backed_up_but_not_synced() {
    use crate::service::data::{backup::BACKUP_DIRS, sync_bundle::SYNC_DIRS};
    assert!(BACKUP_DIRS.contains(&PLUGINS_DIR));
    assert!(!SYNC_DIRS.contains(&PLUGINS_DIR));
}

#[test]
fn version_compare() {
    assert!(manifest::version_newer("1.10.0", "1.9.9"));
    assert!(!manifest::version_newer("1.0", "1.0.0"));
    assert!(manifest::version_newer("v2", "1.99"));
    assert!(!manifest::version_newer("0.2.0", "0.2.0"));
}
