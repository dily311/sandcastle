//! 커뮤니티 플러그인 설치 테스트 (CLAUDE.md 6-D-1)
use std::{
    io::Write,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use zip::{write::SimpleFileOptions, ZipWriter};

use super::*;
use crate::{db, db::conn::DbPool, service::data::trash};

async fn app_dir() -> (tempfile::TempDir, PathBuf, DbPool) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("appdata");
    let db = db::conn::init_pool(base.clone()).await.unwrap();
    (dir, base, db)
}

/// 설치된 플러그인의 (유일한) 판 폴더 안 파일 — 판 폴더 배치(`plugins/<id>/<판>/`)
fn vfile(base: &Path, id: &str, rel: &str) -> std::path::PathBuf {
    let dirs = versions::dirs_of(&base.join("plugins").join(id));
    assert_eq!(dirs.len(), 1, "{id}: 판 폴더가 하나여야 함: {dirs:?}");
    dirs[0].1.join(rel)
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
    assert!(vfile(&base, "word-count", "panel/index.html").is_file());

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
    assert!(vfile(&base, "lint", "main.js").is_file());
    assert!(!vfile(&base, "lint", ".git").exists());

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
        // 화면 선언 (6-D-6): 권한·id·페이지
        ("ui.panel", json!({ "id": "p9", "name": "x", "version": "1", "apiVersion": 1, "panels": [{ "id": "a", "title": "A", "page": "p.html" }] }).to_string()),
        ("ui.command", json!({ "id": "p10", "name": "x", "version": "1", "apiVersion": 1, "commands": [{ "id": "a", "title": "A" }] }).to_string()),
        ("page", json!({ "id": "p11", "name": "x", "version": "1", "apiVersion": 1, "permissions": ["ui.panel"], "panels": [{ "id": "a", "title": "A", "page": "../x.html" }] }).to_string()),
        ("겹침", json!({ "id": "p12", "name": "x", "version": "1", "apiVersion": 1, "permissions": ["ui.command"], "commands": [{ "id": "a", "title": "A" }, { "id": "a", "title": "B" }] }).to_string()),
    ];
    for (want, m) in cases {
        let src = folder(d.path(), &[("manifest.json", &m), ("main.js", "1")]);
        let err = preview(&db, &base, &src).await.expect_err(&m).to_string();
        assert!(err.contains(want), "{m}: {err}");
    }
    // 패널 페이지가 묶음에 없음
    let src = folder(
        d.path(),
        &[(
            "manifest.json",
            &json!({ "id": "p13", "name": "x", "version": "1", "apiVersion": 1, "permissions": ["ui.panel"], "panels": [{ "id": "a", "title": "A", "page": "panel.html" }] }).to_string(),
        ), ("main.js", "1")],
    );
    assert!(preview(&db, &base, &src).await.unwrap_err().to_string().contains("패널"));
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
    type Case<'a> = (&'a str, Vec<(&'a str, &'a [u8])>);
    let bad: Vec<Case> = vec![
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
    assert_eq!(std::fs::read_to_string(base.join("plugins/pp/1.1.0/main.js")).unwrap(), "2");
    assert_eq!(
        std::fs::read_to_string(base.join("plugins/pp/1.0.0/main.js")).unwrap(),
        "1",
        "옛 판은 지우지 않음"
    );

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
    assert!(vfile(&base, "tr", "main.js").is_file());

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

// ---------------------------------------------------------------------------
// 보안 점검(Opus 자문)에서 나온 것들 — ADR-012
// ---------------------------------------------------------------------------

fn manifest_with(id: &str, version: &str, perms: &[&str]) -> String {
    manifest(id, version, perms)
}

#[test]
fn manifest_text_drops_control_and_direction_characters() {
    let raw = json!({ "id": "spoof", "name": "Safe\u{202E}etatS\u{200B}\u{0007} App", "version": "1\u{FEFF}.0", "apiVersion": 1,
        "author": "a\u{2066}b", "description": "d\u{0000}e" });
    let m = manifest::parse(raw.to_string().as_bytes(), "0.2.0").unwrap();
    assert_eq!(m.name, "SafeetatS App");
    assert_eq!(m.version, "1.0");
    assert_eq!(m.author, "ab");
    assert_eq!(m.description, "de");
    // 한글·이모지는 그대로
    assert_eq!(manifest::clean("글자 수 📝"), "글자 수 📝");
}

#[tokio::test]
async fn update_keeps_only_permissions_still_requested() {
    let (d, base, db) = app_dir().await;
    let v1 = folder(
        d.path(),
        &[
            ("manifest.json", &manifest_with("shrink", "1.0.0", &["ai", "storage", "messages.process"])),
            ("main.js", "1"),
        ],
    );
    install_from(&db, &base, &v1).await.unwrap();
    set_enabled(&db, &base, "shrink", true, vec!["ai".into(), "messages.process".into(), "storage".into()])
        .await
        .unwrap();
    runtime::ai_state_for_test_set(&db, "shrink", true, "fix").await;

    // v2: ai·messages.process를 뺀다 → 허락도 교집합, 자동 AI 호출·후처리 방식은 초기화, 켜진 채
    let v2 = folder(
        d.path(),
        &[("manifest.json", &manifest_with("shrink", "2.0.0", &["storage"])), ("main.js", "2")],
    );
    let p = install_from(&db, &base, &v2).await.unwrap();
    assert!(p.enabled);
    assert_eq!(p.granted, ["storage"]);
    assert!(!p.ai_auto);
    assert_eq!(p.process_mode, "notes");

    // v3: ai를 다시 요구 → 예전에 허락했던 것을 되살리지 않는다: 꺼지고, 늘어난 권한으로 보인다
    let v3 = folder(
        d.path(),
        &[("manifest.json", &manifest_with("shrink", "3.0.0", &["ai", "storage"])), ("main.js", "3")],
    );
    let pv = preview(&db, &base, &v3).await.unwrap();
    assert_eq!(pv.added_permissions, ["ai"]);
    let p = install(&db, &base, &v3, &pv.hash).await.unwrap();
    assert!(!p.enabled);
    assert!(!p.granted.contains(&"ai".to_string()));
}

#[tokio::test]
async fn reinstall_without_folder_starts_fresh() {
    // 폴더만 사라지고 행·저장소가 남은 id에 다른 사람의 같은 id 플러그인을 설치하면 이전 상태를 물려받지 않는다
    let (d, base, db) = app_dir().await;
    let a = folder(
        d.path(),
        &[("manifest.json", &manifest_with("same-id", "1.0.0", &["ai", "storage"])), ("main.js", "1")],
    );
    install_from(&db, &base, &a).await.unwrap();
    set_enabled(&db, &base, "same-id", true, vec!["ai".into(), "storage".into()]).await.unwrap();
    runtime::ai_state_for_test_set(&db, "same-id", true, "fix").await;
    storage::set(&db, "same-id", "secret", &json!("old data")).await.unwrap();
    std::fs::remove_dir_all(base.join("plugins/same-id")).unwrap();

    let b = folder(
        d.path(),
        &[("manifest.json", &manifest_with("same-id", "9.0.0", &["ai", "storage"])), ("main.js", "evil")],
    );
    let p = install_from(&db, &base, &b).await.unwrap();
    assert!(!p.enabled && !p.ai_auto && p.granted.is_empty());
    assert_eq!(
        storage::get(&db, "same-id", "secret").await.unwrap(),
        Value::Null,
        "이전 저장소를 물려받지 않음"
    );
}

#[tokio::test]
async fn changed_code_is_not_enabled_until_consent_again() {
    let (d, base, db) = app_dir().await;
    let v1 = folder(d.path(), &[("manifest.json", &manifest_with("hashy", "1", &["storage"])), ("main.js", "good")]);
    install_from(&db, &base, &v1).await.unwrap();
    assert!(set_enabled(&db, &base, "hashy", true, vec!["storage".into()]).await.unwrap().enabled);
    assert!(!get(&db, &base, "hashy").await.unwrap().code_changed);

    // 폴더의 코드를 손으로 바꾸면 켜진 것으로 치지 않는다 (같은 manifest여도)
    std::fs::write(vfile(&base, "hashy", "main.js"), "evil").unwrap();
    let p = get(&db, &base, "hashy").await.unwrap();
    assert!(!p.enabled && p.code_changed);
    // 파일을 더하는 것도 마찬가지
    std::fs::write(vfile(&base, "hashy", "main.js"), "good").unwrap();
    assert!(get(&db, &base, "hashy").await.unwrap().enabled, "원래대로 돌리면 다시 켜진 것");
    std::fs::write(vfile(&base, "hashy", "extra.js"), "x").unwrap();
    assert!(get(&db, &base, "hashy").await.unwrap().code_changed);

    // 다시 켜면(동의) 지금 코드가 기준이 된다
    let p = set_enabled(&db, &base, "hashy", true, vec!["storage".into()]).await.unwrap();
    assert!(p.enabled && !p.code_changed);
}

#[tokio::test]
async fn restored_backup_and_trash_do_not_start_plugins_on_their_own() {
    use crate::service::data::backup;
    let (d, base, db) = app_dir().await;
    let v = folder(d.path(), &[("manifest.json", &manifest_with("rb", "1", &["ai", "storage"])), ("main.js", "1")]);
    install_from(&db, &base, &v).await.unwrap();
    set_enabled(&db, &base, "rb", true, vec!["ai".into(), "storage".into()]).await.unwrap();
    runtime::ai_state_for_test_set(&db, "rb", true, "notes").await;

    // 백업을 복원한 뒤 앱이 처음 켜질 때: 켜짐·자동 AI 호출이 꺼진다 (폴더는 그대로)
    std::fs::write(base.join("restore-fixup.json"), br#"{"oldAppDataDir":""}"#).unwrap();
    backup::finish_restore_fixups(&db, &base).await.unwrap();
    let p = get(&db, &base, "rb").await.unwrap();
    assert!(!p.enabled && !p.ai_auto);
    assert!(vfile(&base, "rb", "main.js").is_file());

    // 휴지통에서 되살린 플러그인도 꺼진 채 (스냅샷 안의 enabled=1을 믿지 않음)
    set_enabled(&db, &base, "rb", true, vec!["ai".into(), "storage".into()]).await.unwrap();
    runtime::ai_state_for_test_set(&db, "rb", true, "notes").await;
    let item = uninstall(&db, &base, "rb").await.unwrap();
    // 되살리기 전에 같은 id를 새로 설치했다면 거절 (저장소가 섞이지 않게)
    let other = folder(d.path(), &[("manifest.json", &manifest_with("rb", "2", &[])), ("main.js", "2")]);
    install_from(&db, &base, &other).await.unwrap();
    let err = trash::restore(&db, &base, &item.id).await.unwrap_err().to_string();
    assert!(err.contains("이미 설치"), "{err}");
    uninstall(&db, &base, "rb").await.unwrap();
    // 비워 놓고 되살리면 꺼진 채로 돌아온다
    let item2 = trash::list(&db).await.unwrap().into_iter().find(|t| t.id == item.id).unwrap();
    trash::restore(&db, &base, &item2.id).await.unwrap();
    let back = get(&db, &base, "rb").await.unwrap();
    assert!(!back.enabled && !back.ai_auto);
}

// ---------------------------------------------------------------------------
// 예제 플러그인 (examples/plugins, CLAUDE.md 6-D-9): 항상 설치 검사를 통과해야 한다
// ---------------------------------------------------------------------------

fn example_dirs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/plugins");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("manifest.json").is_file())
        .collect();
    v.sort();
    v
}

#[tokio::test]
async fn bundled_examples_install_and_follow_the_rules() {
    let dirs = example_dirs();
    assert!(dirs.len() >= 3, "예제가 줄었음: {dirs:?}");
    let (_d, base, db) = app_dir().await;
    for dir in &dirs {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let pv = preview(&db, &base, dir).await.unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(pv.manifest.id, name, "폴더 이름과 id가 같아야 함");
        assert!(!pv.manifest.description.is_empty() && !pv.manifest.author.is_empty(), "{name}: 설명·작성자");
        // 선언한 권한만 쓰는지: 후처리/프롬프트/AI/패널/명령 선언과 main.js의 사용이 맞는다
        let main = std::fs::read_to_string(dir.join(&pv.manifest.main)).unwrap();
        let perms = &pv.manifest.permissions;
        let has = |p: &str| perms.iter().any(|x| x == p);
        assert_eq!(main.contains("sandcastle.ai."), has("ai"), "{name}: ai 권한과 사용이 어긋남");
        assert_eq!(main.contains("\"message.process\""), has("messages.process"), "{name}: messages.process");
        assert_eq!(main.contains("\"prompt.build\""), has("prompt.modify"), "{name}: prompt.modify");
        assert_eq!(!pv.manifest.commands.is_empty(), main.contains("on(\"command\""), "{name}: 명령 선언과 처리");
        assert_eq!(!pv.manifest.commands.is_empty(), has("ui.command"), "{name}: ui.command");
        assert_eq!(!pv.manifest.panels.is_empty(), has("ui.panel"), "{name}: ui.panel");
        for p in &pv.manifest.panels {
            let mut html = std::fs::read_to_string(dir.join(&p.page)).unwrap();
            // 설명 주석은 건너뛴다
            while let (Some(a), Some(b)) = (html.find("<!--"), html.find("-->")) {
                html.replace_range(a..b + 3, "");
            }
            // 패널 페이지는 인라인 스크립트를 쓸 수 없다 (CSP) — <script src=…>만
            for chunk in html.split("<script").skip(1) {
                let tag = chunk.split('>').next().unwrap_or("");
                assert!(tag.contains("src="), "{name}: {} 에 인라인 <script>가 있음", p.page);
            }
        }
        install(&db, &base, dir, &pv.hash).await.unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    // 설치된 것은 꺼진 채
    let all = list(&db, &base).await.unwrap();
    assert_eq!(all.len(), dirs.len());
    assert!(all.iter().all(|p| !p.enabled && p.manifest.is_some() && p.error.is_none()));
}

// ---------------------------------------------------------------------------
// 판 폴더 (1-F-1): plugins/<id>/<판>/, 쓰는 판 고르기(되돌리기), 판 폴더 지우기
// ---------------------------------------------------------------------------

#[tokio::test]
async fn updates_keep_old_versions_and_the_user_picks_which_one_to_use() {
    let (d, base, db) = app_dir().await;
    let v1 = folder(d.path(), &[("manifest.json", &manifest("vp", "1.0.0", &["storage"])), ("main.js", "one")]);
    install_from(&db, &base, &v1).await.unwrap();
    set_enabled(&db, &base, "vp", true, vec!["storage".into()]).await.unwrap();
    storage::set(&db, "vp", "k", &json!("a")).await.unwrap();

    // 업데이트: 새 판 폴더가 생기고 옛 판은 남는다. 권한이 그대로면 켜진 채 이어받음, 저장소도 이어받음
    let v2 = folder(d.path(), &[("manifest.json", &manifest("vp", "1.1.0", &["storage"])), ("main.js", "two")]);
    let p = install_from(&db, &base, &v2).await.unwrap();
    assert_eq!((p.versions.len(), p.active_dir.as_str(), p.enabled), (2, "1.1.0", true));
    assert!(base.join("plugins/vp/1.0.0/main.js").is_file() && base.join("plugins/vp/1.1.0/main.js").is_file());
    assert_eq!(
        p.versions.iter().filter(|v| v.active).map(|v| v.version.as_str()).collect::<Vec<_>>(),
        ["1.1.0"]
    );
    assert_eq!(storage::get(&db, "vp", "k").await.unwrap(), json!("a"));
    storage::set(&db, "vp", "k", &json!("b")).await.unwrap();

    // 되돌리기: 꺼진 채로(코드가 바뀜), 저장소는 그 판을 떠날 때의 값으로
    let back = switch_version(&db, &base, "vp", "1.0.0").await.unwrap();
    assert_eq!((back.active_dir.as_str(), back.enabled, back.ai_auto), ("1.0.0", false, false));
    assert_eq!(back.manifest.as_ref().map(|m| m.version.as_str()), Some("1.0.0"));
    assert_eq!(storage::get(&db, "vp", "k").await.unwrap(), json!("a"), "옛 판의 저장소로 복원");
    // 다시 켜서 동의하면 쓸 수 있고, 판 폴더의 보관 파일은 코드 지문에 들어가지 않는다
    let on = set_enabled(&db, &base, "vp", true, vec!["storage".into()]).await.unwrap();
    assert!(on.enabled && !on.code_changed);
    // 새 판으로 다시: 새 판을 떠날 때 보관한 값(b)
    let fwd = switch_version(&db, &base, "vp", "1.1.0").await.unwrap();
    assert_eq!((fwd.active_dir.as_str(), fwd.enabled), ("1.1.0", false));
    assert_eq!(storage::get(&db, "vp", "k").await.unwrap(), json!("b"));
    assert!(base.join("plugins/vp/1.0.0").join(versions::STORAGE_SNAPSHOT).is_file());

    // 지우기: 쓰는 판은 거절, 쓰지 않는 판은 지워짐, 폴더 밖·없는 폴더는 거절
    assert!(delete_version(&db, &base, "vp", "1.1.0").await.is_err());
    for bad in ["..", "../x", ".", "", "nope", "1.0.0/../1.1.0"] {
        assert!(
            delete_version(&db, &base, "vp", bad).await.is_err() && switch_version(&db, &base, "vp", bad).await.is_err(),
            "{bad}"
        );
    }
    let after = delete_version(&db, &base, "vp", "1.0.0").await.unwrap();
    assert_eq!(after.versions.len(), 1);
    assert!(!base.join("plugins/vp/1.0.0").exists() && base.join("plugins/vp/1.1.0/main.js").is_file());
}

#[tokio::test]
async fn same_version_is_reused_when_the_code_matches_and_split_when_it_does_not() {
    let (d, base, db) = app_dir().await;
    let a = folder(d.path(), &[("manifest.json", &manifest("sv", "1.0.0", &[])), ("main.js", "one")]);
    install_from(&db, &base, &a).await.unwrap();
    // 같은 판·같은 코드를 다시 설치해도 폴더가 늘지 않는다
    assert_eq!(install_from(&db, &base, &a).await.unwrap().versions.len(), 1);
    // 같은 판 이름에 다른 코드: 코드 지문을 붙인 다른 폴더, 옛 것은 그대로 (같은 코드를 또 설치하면 그 폴더를 다시 씀)
    let b = folder(d.path(), &[("manifest.json", &manifest("sv", "1.0.0", &[])), ("main.js", "patched")]);
    let p = install_from(&db, &base, &b).await.unwrap();
    assert_eq!(p.versions.len(), 2);
    assert!(p.active_dir.starts_with("1.0.0-") && std::fs::read_to_string(base.join("plugins/sv/1.0.0/main.js")).unwrap() == "one");
    assert_eq!(install_from(&db, &base, &b).await.unwrap().versions.len(), 2);
    // 판 문자열이 폴더 이름으로 쓸 수 없는 글자를 가져도 안전하게
    assert_eq!(versions::dir_name("v1/../x y"), "v1_.._x_y");
    assert_eq!(versions::dir_name("..."), "v");
    assert!(
        versions::valid_dir_name("1.0.0-ab12") && !versions::valid_dir_name("..") && !versions::valid_dir_name(".hidden") && !versions::valid_dir_name("a/b")
    );
}

#[tokio::test]
async fn the_old_flat_layout_is_moved_into_a_version_folder_once() {
    let (_d, base, db) = app_dir().await;
    let flat = base.join("plugins/old-flat");
    std::fs::create_dir_all(flat.join("panel")).unwrap();
    std::fs::write(flat.join("manifest.json"), manifest("old-flat", "2.3.0", &[])).unwrap();
    std::fs::write(flat.join("main.js"), "x").unwrap();
    std::fs::write(flat.join("panel/p.html"), "<p>").unwrap();

    let all = list(&db, &base).await.unwrap();
    let p = all.iter().find(|p| p.id == "old-flat").unwrap();
    assert_eq!((p.active_dir.as_str(), p.versions.len(), p.manifest.is_some()), ("2.3.0", 1, true));
    assert!(base.join("plugins/old-flat/2.3.0/panel/p.html").is_file() && !base.join("plugins/old-flat/manifest.json").exists());
    // 다시 불러도 그대로 (두 번 옮기지 않음)
    assert_eq!(versions::migrate_layout(&base).unwrap(), 0);
    assert_eq!(list(&db, &base).await.unwrap().iter().find(|p| p.id == "old-flat").unwrap().versions.len(), 1);
}

#[test]
fn sdk_support_range_is_consistent() {
    const { assert!(manifest::MIN_API_VERSION >= 1 && manifest::MIN_API_VERSION <= manifest::API_VERSION) };
}
