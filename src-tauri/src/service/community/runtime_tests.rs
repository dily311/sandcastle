//! 플러그인 런타임 테스트 (CLAUDE.md 6-D-2). 도는 목록이 전역이라 한 테스트 안에서 차례로 확인한다.
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use tauri::http::Request;

use super::*;
use crate::{db, db::conn::DbPool, service::data::trash};

async fn app_dir() -> (tempfile::TempDir, PathBuf, DbPool) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("appdata");
    let db = db::conn::init_pool(base.clone()).await.unwrap();
    (dir, base, db)
}

async fn add(db: &DbPool, base: &Path, root: &Path, id: &str, perms: &[&str], enable: bool) {
    let src = root.join(format!("src-{id}"));
    std::fs::create_dir_all(src.join("img")).unwrap();
    std::fs::write(
        src.join("manifest.json"),
        json!({ "id": id, "name": id, "version": "1.0.0", "apiVersion": 1, "permissions": perms }).to_string(),
    )
    .unwrap();
    std::fs::write(src.join("main.js"), "sandcastle.log('hi')").unwrap();
    std::fs::write(src.join("img/a.png"), b"\x89PNG").unwrap();
    let p = preview(db, base, &src).await.unwrap();
    install(db, base, &src, &p.hash).await.unwrap();
    if enable {
        set_enabled(db, base, id, true, perms.iter().map(|s| s.to_string()).collect()).await.unwrap();
    }
}

fn get_url(path: &str) -> (u16, String, String) {
    let req = Request::builder().uri(format!("{}{path}", runtime::origin())).body(vec![]).unwrap();
    let res = runtime::handle(&req);
    let csp = res
        .headers()
        .get("Content-Security-Policy")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    (res.status().as_u16(), String::from_utf8_lossy(res.body()).into_owned(), csp)
}

async fn run_start(db: &DbPool, base: &Path) -> runtime::RuntimeStart {
    runtime::start(&list(db, base).await.unwrap(), base)
}

#[tokio::test]
async fn runtime_serves_and_checks_calls() {
    let (d, base, db) = app_dir().await;
    add(&db, &base, d.path(), "rt-store", &["storage"], true).await;
    add(&db, &base, d.path(), "rt-plain", &[], true).await;
    add(&db, &base, d.path(), "rt-off", &["storage"], false).await;

    // 시작 전에는 아무것도 주지 않는다 (Lazy)
    runtime::stop_all();
    assert_eq!(get_url("/rt-store/__host").0, 404);

    let started = run_start(&db, &base).await;
    let ids: Vec<&str> = started.plugins.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["rt-plain", "rt-store"], "꺼진 플러그인은 시작하지 않음");
    assert_eq!(started.plugins[1].host_url, runtime::host_url("rt-store"));
    assert_eq!(started.api.get("storage.set"), Some(&Some("storage".to_string())));

    // 서빙
    let (status, body, csp) = get_url("/rt-store/__host");
    assert_eq!(status, 200);
    assert!(body.contains("/__sdk.js") && body.contains("/rt-store/main.js"), "{body}");
    assert!(csp.contains("connect-src 'none'") && csp.contains("frame-src 'none'"), "{csp}");
    assert!(csp.contains("script-src") && !csp.contains("'unsafe-eval'") && !csp.contains("https:"), "{csp}");
    assert!(
        csp.contains(&format!("{}/rt-store/", runtime::origin())) && !csp.contains("rt-plain"),
        "자기 경로만: {csp}"
    );
    assert_eq!(get_url("/__sdk.js").0, 200);
    assert_eq!(get_url("/rt-store/main.js").0, 200);
    assert_eq!(get_url("/rt-store/img/a.png").0, 200);
    for bad in [
        "/rt-store/../rt-plain/main.js",
        "/rt-store/%2e%2e/rt-plain/main.js",
        "/rt-store/manifest.json/../main.js",
        "/rt-off/main.js",
        "/nope/main.js",
        "/rt-store/missing.js",
    ] {
        assert_eq!(get_url(bad).0, 404, "{bad}");
    }

    // 호출: 권한·메서드·저장소
    assert!(call(&db, "rt-store", "storage.set", &json!({ "key": "n", "value": { "a": 1 } })).await.is_ok());
    assert_eq!(call(&db, "rt-store", "storage.get", &json!({ "key": "n" })).await.unwrap(), json!({ "a": 1 }));
    assert_eq!(call(&db, "rt-store", "storage.keys", &Value::Null).await.unwrap(), json!(["n"]));
    let info = call(&db, "rt-store", "app.info", &Value::Null).await.unwrap();
    assert_eq!(info["pluginId"], "rt-store");
    assert_eq!(info["permissions"], json!(["storage"]));
    let err = call(&db, "rt-plain", "storage.get", &json!({ "key": "n" })).await.unwrap_err().to_string();
    assert!(err.contains("권한"), "{err}");
    assert!(call(&db, "rt-store", "fs.read", &Value::Null).await.is_err());
    assert!(call(&db, "rt-off", "log", &json!({ "message": "x" })).await.is_err(), "도는 중이 아닌 플러그인");
    let big = "x".repeat(storage::MAX_VALUE_BYTES + 10);
    assert!(call(&db, "rt-store", "storage.set", &json!({ "key": "big", "value": big })).await.is_err());
    // 플러그인끼리 저장소가 섞이지 않는다
    sqlx::query("INSERT INTO user_plugin_storage (plugin_id, key, value) VALUES ('rt-plain', 'n', '\"other\"')")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(call(&db, "rt-store", "storage.get", &json!({ "key": "n" })).await.unwrap(), json!({ "a": 1 }));
    call(&db, "rt-plain", "log", &json!({ "level": "warn", "message": "조심" })).await.unwrap();
    assert!(runtime::log_of("rt-plain").iter().any(|l| l.level == "warn" && l.message == "조심"));

    // 끄면 곧바로 내린다
    set_enabled(&db, &base, "rt-plain", false, vec![]).await.unwrap();
    assert_eq!(get_url("/rt-plain/main.js").0, 404);
    assert!(call(&db, "rt-plain", "log", &json!({ "message": "x" })).await.is_err());

    // 오류가 많으면 멈춤 → 다시 시작해도 빠짐 → 다시 켜면 풀림
    for i in 0..30 {
        let _ = call(&db, "rt-store", "report", &json!({ "message": format!("boom {i}") })).await;
    }
    assert!(runtime::is_suspended("rt-store"));
    assert_eq!(get_url("/rt-store/__host").0, 404);
    assert!(call(&db, "rt-store", "storage.keys", &Value::Null).await.is_err());
    assert!(run_start(&db, &base).await.plugins.iter().all(|p| p.id != "rt-store"));
    set_enabled(&db, &base, "rt-store", true, vec!["storage".into()]).await.unwrap();
    assert!(!runtime::is_suspended("rt-store"));
    assert!(run_start(&db, &base).await.plugins.iter().any(|p| p.id == "rt-store"));

    // manifest가 허락보다 많은 권한을 달라고 바뀌면(손으로 고침) 시작하지 않는다
    std::fs::write(
        base.join("plugins/rt-store/manifest.json"),
        json!({ "id": "rt-store", "name": "x", "version": "1.0.0", "apiVersion": 1, "permissions": ["storage", "ai"] }).to_string(),
    )
    .unwrap();
    assert!(run_start(&db, &base).await.plugins.iter().all(|p| p.id != "rt-store"));
    std::fs::write(
        base.join("plugins/rt-store/manifest.json"),
        json!({ "id": "rt-store", "name": "x", "version": "1.0.0", "apiVersion": 1, "permissions": ["storage"] }).to_string(),
    )
    .unwrap();

    // 지우면 저장소도 휴지통으로, 되살리면 같이
    let item = uninstall(&db, &base, "rt-store").await.unwrap();
    assert_eq!(get_url("/rt-store/main.js").0, 404);
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_plugin_storage WHERE plugin_id = 'rt-store'")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(left, 0);
    trash::restore(&db, &base, &item.id).await.unwrap();
    let back: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_plugin_storage WHERE plugin_id = 'rt-store'")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(back, 1);

    runtime::stop_all();
}

#[test]
fn every_api_permission_is_a_known_permission() {
    for (m, p) in runtime::API {
        if let Some(p) = p {
            assert!(manifest::known_permission(p), "{m}: {p}");
        }
    }
}
