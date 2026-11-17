//! 동기화 병합 테스트 (2-B-2): 기기 둘(각자 DB·앱 데이터 폴더)이 묶음을 주고받는다
use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::{Map, Value};

use super::sync_bundle::{self, BundleOptions};
use super::sync_merge::{merge, BundleData, MergeReport};
use crate::db::conn::DbPool;

struct Dev {
    _dir: tempfile::TempDir,
    base: PathBuf,
    db: DbPool,
    id: String,
}

async fn dev(id: &str) -> Dev {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("appdata");
    let db = crate::db::conn::init_pool(base.clone()).await.unwrap();
    Dev {
        _dir: dir,
        base,
        db,
        id: id.into(),
    }
}

async fn tick() {
    tokio::time::sleep(std::time::Duration::from_millis(4)).await;
}

async fn read(from: &Dev, to: &Dev) -> BundleData {
    tick().await;
    let out = from.base.parent().unwrap().join(format!("to-{}.scsync", to.id));
    sync_bundle::write_bundle(&from.db, &from.base, &Map::new(), &from.id, &BundleOptions::default(), &out)
        .await
        .unwrap();
    let local = sync_bundle::schema_version(&to.db).await.unwrap();
    let mut b = sync_bundle::open(&out, local, &to.base).unwrap();
    BundleData::read(&mut b, &to.db).await.unwrap()
}

/// from의 전체 묶음을 to에 합친다
async fn send(from: &Dev, to: &Dev) -> MergeReport {
    let data = read(from, to).await;
    let r = merge(&to.db, &data, &to.id, &mut Map::new(), false).await.unwrap();
    tick().await;
    r
}

async fn exec(d: &Dev, sql: &str) {
    sqlx::query(sqlx::AssertSqlSafe(sql.to_string())).execute(&d.db).await.unwrap();
    tick().await;
}

/// 비교할 상태: 캐릭터·대화·메시지 (시각 빼고)
async fn state(d: &Dev) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (sql, tag) in [
        ("SELECT id, name FROM characters", "char"),
        ("SELECT id, title FROM conversations", "conv"),
        ("SELECT id, conversation_id || '#' || seq || ':' || content FROM messages", "msg"),
    ] {
        let rows: Vec<(String, String)> = sqlx::query_as(sql).fetch_all(&d.db).await.unwrap();
        for (k, v) in rows {
            out.insert(format!("{tag}:{k}"), v);
        }
    }
    out
}

async fn seed(a: &Dev) {
    exec(a, "INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c1', 'Aria', 1, 0)").await;
    exec(
        a,
        "INSERT INTO conversations (id, title, character_id, mode, created_at, updated_at) VALUES ('v1', '숲', 'c1', 'chat', 1, 0)",
    )
    .await;
    exec(
        a,
        "INSERT INTO messages (id, conversation_id, seq, role, content, created_at, updated_at) VALUES ('m1', 'v1', 1, 'user', '안녕', 1, 0)",
    )
    .await;
    exec(
        a,
        "INSERT INTO messages (id, conversation_id, seq, role, content, created_at, updated_at) VALUES ('m2', 'v1', 2, 'assistant', '반가워', 1, 0)",
    )
    .await;
}

/// A가 만든 것을 B에 보내고 다시 돌려받아, 두 기기가 서로를 아는 상태로
async fn paired() -> (Dev, Dev) {
    let (a, b) = (dev("dev-a").await, dev("dev-b").await);
    seed(&a).await;
    let r = send(&a, &b).await;
    assert_eq!(r.conflicts, 0, "처음 받는 빈 기기에 충돌이 나면 안 됨 (기본 페르소나·프리셋 포함): {r:?}");
    assert!(r.added >= 4, "{r:?}");
    let r = send(&b, &a).await;
    assert_eq!((r.conflicts, r.added, r.updated), (0, 0, 0), "같은 내용이 돌아오면 아무것도 바뀌지 않음: {r:?}");
    assert_eq!(state(&a).await, state(&b).await);
    (a, b)
}

#[tokio::test]
async fn separate_edits_converge_in_both_directions() {
    let (a, b) = paired().await;
    exec(&a, "UPDATE characters SET name = 'Aria2' WHERE id = 'c1'").await;
    exec(&b, "INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c2', 'Bea', 1, 0)").await;
    exec(&b, "UPDATE conversations SET title = '깊은 숲' WHERE id = 'v1'").await;

    let r = send(&a, &b).await;
    assert_eq!((r.updated, r.conflicts), (1, 0), "{r:?}");
    let r = send(&b, &a).await;
    assert_eq!((r.added, r.updated, r.conflicts), (1, 1, 0), "{r:?}");

    let (sa, sb) = (state(&a).await, state(&b).await);
    assert_eq!(sa, sb, "양쪽 결과가 같아야 함");
    assert_eq!(sa["char:c1"], "Aria2");
    assert_eq!(sa["char:c2"], "Bea");
    assert_eq!(sa["conv:v1"], "깊은 숲");

    // 반대 순서(B 먼저)로 해도 같다
    let (a, b) = paired().await;
    exec(&a, "UPDATE characters SET name = 'Aria2' WHERE id = 'c1'").await;
    exec(&b, "UPDATE conversations SET title = '깊은 숲' WHERE id = 'v1'").await;
    send(&b, &a).await;
    send(&a, &b).await;
    assert_eq!(state(&a).await, state(&b).await);
}

#[tokio::test]
async fn same_row_edited_on_both_keeps_both() {
    let (a, b) = paired().await;
    exec(&a, "UPDATE characters SET name = 'A가 고침' WHERE id = 'c1'").await;
    exec(&b, "UPDATE characters SET name = 'B가 고침' WHERE id = 'c1'").await;

    let r = send(&a, &b).await;
    assert_eq!(r.conflicts, 1, "{r:?}");
    assert_eq!(state(&b).await["char:c1"], "B가 고침", "이 기기 것은 그대로");
    let (kind, remote): (String, String) = sqlx::query_as("SELECT kind, remote_row FROM sync_conflicts").fetch_one(&b.db).await.unwrap();
    assert_eq!(kind, "edit");
    assert!(remote.contains("A가 고침"), "받은 것은 충돌 목록에 남음");

    let r = send(&b, &a).await;
    assert_eq!(r.conflicts, 1);
    assert_eq!(state(&a).await["char:c1"], "A가 고침");
}

#[tokio::test]
async fn delete_vs_edit_keeps_the_edit_with_its_messages() {
    let (a, b) = paired().await;
    exec(&a, "DELETE FROM conversations WHERE id = 'v1'").await; // 메시지도 CASCADE
    exec(&b, "UPDATE conversations SET title = '이어 쓸 숲' WHERE id = 'v1'").await;

    // B: A의 지우기를 받지만 B가 고쳤으므로 대화와 그 메시지가 남는다
    let r = send(&a, &b).await;
    assert!(r.kept_local_edits >= 3, "{r:?}");
    assert_eq!(r.deleted, 0);
    let sb = state(&b).await;
    assert_eq!(sb["conv:v1"], "이어 쓸 숲");
    assert!(sb.contains_key("msg:m1") && sb.contains_key("msg:m2"));

    // A: B의 고침이 A의 지우기를 이기고, 메시지도 같이 돌아온다
    let r = send(&b, &a).await;
    assert!(r.restored >= 3, "{r:?}");
    assert_eq!(state(&a).await, state(&b).await);
    let tomb: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_tombstones WHERE tbl IN ('conversations', 'messages')")
        .fetch_one(&a.db)
        .await
        .unwrap();
    assert_eq!(tomb, 0, "되살리면 지운 기록도 사라짐");
}

#[tokio::test]
async fn untouched_delete_is_applied() {
    let (a, b) = paired().await;
    exec(&a, "DELETE FROM conversations WHERE id = 'v1'").await;
    let r = send(&a, &b).await;
    assert!(r.deleted >= 1, "{r:?}");
    assert_eq!(state(&a).await, state(&b).await);
    assert!(!state(&b).await.contains_key("msg:m1"));
}

#[tokio::test]
async fn both_continuing_a_chat_makes_a_branch() {
    let (a, b) = paired().await;
    exec(
        &a,
        "INSERT INTO messages (id, conversation_id, seq, role, content, created_at, updated_at) VALUES ('ma', 'v1', 3, 'user', 'A의 다음 말', 1, 0)",
    )
    .await;
    exec(
        &b,
        "INSERT INTO messages (id, conversation_id, seq, role, content, created_at, updated_at) VALUES ('mb', 'v1', 3, 'user', 'B의 다음 말', 1, 0)",
    )
    .await;
    exec(&b, "INSERT INTO message_bookmarks (id, message_id, created_at) VALUES ('bk', 'mb', 1)").await;

    let r = send(&b, &a).await;
    assert_eq!(r.branches, 1, "{r:?}");
    let s = state(&a).await;
    assert_eq!(s["msg:ma"], "v1#3:A의 다음 말", "이 기기의 대화는 그대로");
    let branch: String = sqlx::query_scalar("SELECT id FROM conversations WHERE id <> 'v1'")
        .fetch_one(&a.db)
        .await
        .unwrap();
    let msgs: Vec<(i64, String)> = sqlx::query_as("SELECT seq, content FROM messages WHERE conversation_id = ? ORDER BY seq")
        .bind(&branch)
        .fetch_all(&a.db)
        .await
        .unwrap();
    assert_eq!(
        msgs,
        vec![(1, "안녕".into()), (2, "반가워".into()), (3, "B의 다음 말".into())],
        "공통 부분 + 받은 이어 쓰기"
    );
    let title: String = sqlx::query_scalar("SELECT title FROM conversations WHERE id = ?")
        .bind(&branch)
        .fetch_one(&a.db)
        .await
        .unwrap();
    assert!(title.contains("다른 기기"));
    // 받은 메시지를 가리키던 책갈피도 분기의 새 메시지를 가리킨다
    let bk_conv: String = sqlx::query_scalar("SELECT m.conversation_id FROM message_bookmarks b JOIN messages m ON m.id = b.message_id WHERE b.id = 'bk'")
        .fetch_one(&a.db)
        .await
        .unwrap();
    assert_eq!(bk_conv, branch);
    // 같은 묶음 내용을 다시 받아도 분기를 또 만들지 않는다
    exec(&b, "UPDATE characters SET name = 'Aria' WHERE id = 'c1'").await;
    let r = send(&b, &a).await;
    assert_eq!(r.branches, 0, "{r:?}");
}

#[tokio::test]
async fn refuses_newer_schema_own_bundle_and_replays_and_dry_run_changes_nothing() {
    let (a, b) = (dev("dev-a").await, dev("dev-b").await);
    seed(&a).await;
    let mut data = read(&a, &b).await;

    // 미리보기: 숫자는 나오지만 DB는 그대로
    let before = state(&b).await;
    let r = merge(&b.db, &data, &b.id, &mut Map::new(), true).await.unwrap();
    assert!(r.dry_run && r.added >= 4);
    assert_eq!(state(&b).await, before);
    let peers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_peers").fetch_one(&b.db).await.unwrap();
    assert_eq!(peers, 0);

    // 자기 묶음
    assert!(merge(&a.db, &data, &a.id, &mut Map::new(), false).await.is_err());
    // 이미 합친 묶음
    merge(&b.db, &data, &b.id, &mut Map::new(), false).await.unwrap();
    assert!(merge(&b.db, &data, &b.id, &mut Map::new(), false)
        .await
        .unwrap_err()
        .to_string()
        .contains("이미 합친"));
    // 스키마가 더 새 앱의 묶음 → 업데이트 안내
    data.manifest.schema_version += 1;
    data.manifest.created_at += 1;
    let err = merge(&b.db, &data, &b.id, &mut Map::new(), false).await.unwrap_err().to_string();
    assert!(err.contains("업데이트"), "{err}");
}

#[tokio::test]
async fn shared_settings_merge_and_device_fields_stay() {
    use crate::models::settings::{AppSetting, SettingsDomain};
    let (a, b) = (dev("dev-a").await, dev("dev-b").await);
    let mut sa = Map::new();
    let mut app = serde_json::to_value(AppSetting::default()).unwrap();
    app["selectedPersonaId"] = "me".into();
    app["selectedApiProfile"] = "p-a".into();
    sa.insert(AppSetting::STORE_KEY.into(), app);
    let out = a.base.parent().unwrap().join("s.scsync");
    sync_bundle::write_bundle(&a.db, &a.base, &sa, &a.id, &BundleOptions::default(), &out)
        .await
        .unwrap();
    let mut bundle = sync_bundle::open(&out, 9999, &b.base).unwrap();
    let data = BundleData::read(&mut bundle, &b.db).await.unwrap();

    let mut sb = Map::new();
    let mut app_b = serde_json::to_value(AppSetting::default()).unwrap();
    app_b["selectedApiProfile"] = "p-b".into();
    sb.insert(AppSetting::STORE_KEY.into(), app_b);
    let r = merge(&b.db, &data, &b.id, &mut sb, false).await.unwrap();
    assert!(r.settings_applied);
    assert_eq!(sb["app-setting"]["selectedPersonaId"], Value::from("me"));
    assert_eq!(sb["app-setting"]["selectedApiProfile"], Value::from("p-b"), "기기 전용은 그대로");
}

/// 에뮬레이터 점검에서 찾음: A → B로 보낸 뒤 B가 고치고 지워서 처음으로 B → A. A는 B에게서 받은 적이 없지만,
/// B의 묶음이 "A의 묶음을 이 시각까지 합쳤다"(acks)를 알려 주므로 A는 고침을 받아들이고 지우기도 적용한다
#[tokio::test]
async fn first_reply_applies_edits_and_deletes_thanks_to_acks() {
    let (a, b) = (dev("dev-a").await, dev("dev-b").await);
    seed(&a).await;
    exec(&a, "INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c9', 'Gone', 1, 0)").await;
    send(&a, &b).await;
    exec(&b, "UPDATE conversations SET title = 'B가 고침' WHERE id = 'v1'").await;
    exec(&b, "DELETE FROM characters WHERE id = 'c9'").await;

    let r = send(&b, &a).await;
    assert_eq!((r.conflicts, r.kept_local_edits), (0, 0), "{r:?}");
    assert!(r.updated >= 1 && r.deleted >= 1, "{r:?}");
    let s = state(&a).await;
    assert_eq!(s["conv:v1"], "B가 고침");
    assert!(!s.contains_key("char:c9"));
    assert_eq!(s, state(&b).await);

    // A가 그 사이 고친 것은 여전히 둘 다 남김
    exec(&a, "UPDATE characters SET name = 'A만 고침' WHERE id = 'c1'").await;
    exec(&b, "UPDATE characters SET name = 'B만 고침' WHERE id = 'c1'").await;
    assert_eq!(send(&b, &a).await.conflicts, 1);
}
