//! 데이터 관리 왕복 테스트 (실제 SQLite 파일 + SillyTavern 샘플)
use std::path::{Path, PathBuf};

use super::{backup, overview, pack};
use crate::{
    db::{self, conn::DbPool},
    models::regex::{RegexPlacement, RegexScope, RegexScriptInput, RegexTarget},
    service::{
        chat::chat_images::add_images,
        import::sillytavern::{import_file, SillyTavernImport},
    },
};

fn sample(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sillyTavern").join(rel)
}

fn chat_sample() -> PathBuf {
    std::fs::read_dir(sample("chats/Seraphina"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .unwrap()
}

/// 앱 데이터 폴더 하나에 Seraphina(카드 + 이미지 3장 + 정규식) + 채팅을 채운다. (캐릭터 id 반환)
async fn populate(db: &DbPool, base: &Path) -> String {
    let SillyTavernImport::Character { character, .. } =
        import_file(db, base, &sample("character/default_Seraphina.png"), None).await.unwrap()
    else {
        panic!()
    };
    let sprites: Vec<PathBuf> = ["joy", "anger", "neutral"]
        .iter()
        .map(|n| sample(&format!("character/Seraphina/{n}.png")))
        .collect();
    add_images(db, base, &character.id, &sprites).await.unwrap();
    db::regex::upsert_regex_script(
        db,
        &RegexScriptInput {
            id: None,
            name: "trim".into(),
            find_regex: "/\\s+$/".into(),
            replace_string: String::new(),
            trim_strings: vec![],
            placement: vec![RegexPlacement::AiOutput],
            target: RegexTarget::Stored,
            scope: RegexScope::Character,
            scope_id: Some(character.id.clone()),
            substitute_macros: Default::default(),
            run_on_edit: true,
            min_depth: None,
            max_depth: None,
            enabled: true,
            sort_order: 0,
            source_format: None,
            extra_json: None,
        },
    )
    .await
    .unwrap();
    import_file(db, base, &chat_sample(), None).await.unwrap();
    character.id
}

async fn app_dir() -> (tempfile::TempDir, PathBuf, DbPool) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("appdata");
    let db = db::conn::init_pool(base.clone()).await.unwrap();
    (dir, base, db)
}

#[tokio::test]
async fn overview_groups_chats_by_character() {
    let (_d, base, db) = app_dir().await;
    let cid = populate(&db, &base).await;

    // 캐릭터가 지워진 대화 하나 (고아)
    sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES ('gone', 'Gone', 0, 0)")
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO conversations (id, title, character_id, mode, created_at, updated_at) VALUES ('orphan', 'o', 'gone', 'chat', 1, 1)")
        .execute(&db)
        .await
        .unwrap();
    db::conversation::delete_character(&db, "gone").await.unwrap();

    let o = overview::data_overview(&db, &base).await.unwrap();
    assert_eq!(o.characters.len(), 1);
    let c = &o.characters[0];
    assert_eq!(c.id, cid);
    assert_eq!((c.conversation_count, c.message_count, c.image_count, c.world_info_book_count), (1, 12, 3, 1));
    assert_eq!(c.conversations[0].hidden_count, 3);
    assert!(c.last_activity.is_some());
    assert_eq!(o.orphan_conversations.len(), 1);
    assert_eq!(o.totals.messages, 12);
    assert!(o.storage.characters_bytes > 0 && o.storage.database_bytes > 0);

    assert_eq!(overview::delete_conversations(&db, &["orphan".into(), "missing".into()]).await.unwrap(), 1);
}

#[tokio::test]
async fn character_pack_round_trip_merges_with_new_ids() {
    let (_a, base_a, db_a) = app_dir().await;
    let old_id = populate(&db_a, &base_a).await;
    let out = base_a.join("exports/seraphina.scpack");
    let summary = pack::export_pack(&db_a, &base_a, &[old_id.clone()], None, &out).await.unwrap();
    assert_eq!((summary.character_count, summary.conversation_count, summary.message_count), (1, 1, 12));

    let (_b, base_b, db_b) = app_dir().await;
    let imported = pack::import_pack(&db_b, &base_b, &out).await.unwrap();
    assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
    assert_eq!((imported.conversation_count, imported.message_count), (1, 12));
    let new_id = imported.characters[0].new_id.clone();
    assert_ne!(new_id, old_id);

    let o = overview::data_overview(&db_b, &base_b).await.unwrap();
    let c = &o.characters[0];
    assert_eq!((c.name.as_str(), c.message_count, c.image_count, c.world_info_book_count), ("Seraphina", 12, 3, 1));
    assert_eq!(c.conversations[0].hidden_count, 3, "숨김 표시도 그대로");

    // 참조가 새 id로 바뀌었는지
    let msgs = db::conversation::get_messages(&db_b, &c.conversations[0].id, 100, None).await.unwrap();
    assert_eq!(msgs[0].character_id.as_deref(), Some(new_id.as_str()));
    assert_eq!(msgs[0].meta.as_ref().unwrap()["sender"]["id"], new_id.as_str());
    let regex = db::regex::get_regex_scripts_for(&db_b, Some(&new_id), None).await.unwrap();
    assert_eq!(regex.len(), 1);
    let character = db::conversation::get_character(&db_b, &new_id).await.unwrap();
    let avatar = PathBuf::from(character.avatar.unwrap());
    assert!(avatar.starts_with(&base_b) && avatar.is_file());
    let books = db::conversation::get_all_worldinfo_entries(&db_b, &character.world_info_ids[0]).await.unwrap();
    assert_eq!(books.len(), 4);
    let images = db::character_image::get_character_images(&db_b, &new_id, false).await.unwrap();
    assert!(images.iter().all(|i| base_b.join(&i.file_path).is_file()));
    assert_eq!(images.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(), vec!["joy", "anger", "neutral"]);

    // 한 번 더 가져오면 덮어쓰지 않고 하나 더 생긴다
    pack::import_pack(&db_b, &base_b, &out).await.unwrap();
    assert_eq!(overview::totals(&db_b).await.unwrap().characters, 2);
    assert!(!base_b.join("tmp").read_dir().map(|mut d| d.next().is_some()).unwrap_or(false), "임시 파일 정리");
}

#[tokio::test]
async fn pack_rejects_conversations_of_other_characters() {
    let (_a, base, db) = app_dir().await;
    let cid = populate(&db, &base).await;
    let err = pack::export_pack(&db, &base, &[cid], Some(&["nope".to_string()]), &base.join("x.scpack"))
        .await
        .err()
        .unwrap();
    assert!(err.to_response().message.contains("고른 캐릭터의 대화가 아닙니다"));
}

#[tokio::test]
async fn full_backup_restores_into_another_app_dir() {
    let (_a, base_a, db_a) = app_dir().await;
    populate(&db_a, &base_a).await;
    std::fs::write(base_a.join("settings.json"), r#"{"app-setting":{"language":"en"}}"#).unwrap();
    let before = overview::totals(&db_a).await.unwrap();
    let out = base_a.join("exports/full.scbackup");
    let summary = backup::export_backup(&db_a, &base_a, &out).await.unwrap();
    // db + settings + 이미지 3장(원본·썸네일) + 아바타
    assert_eq!(summary.file_count, 1 + 1 + 3 * 2 + 1);

    // 다른 PC: 기존 데이터가 조금 있는 상태
    let dir_b = tempfile::tempdir().unwrap();
    let base_b = dir_b.path().join("appdata");
    {
        let db_b = db::conn::init_pool(base_b.clone()).await.unwrap();
        std::fs::create_dir_all(base_b.join("characters")).unwrap();
        std::fs::write(base_b.join("characters/old.txt"), "old").unwrap();
        let preview = backup::prepare_restore(&base_b, &out).await.unwrap();
        assert_eq!(preview.totals.messages, before.messages);
        assert!(!preview.will_migrate);
        assert!(backup::pending_restore(&base_b).is_some());
        db_b.close().await; // 앱 종료
    }

    // 재시작
    assert!(backup::apply_pending_restore(&base_b).unwrap());
    assert!(backup::pending_restore(&base_b).is_none());
    let db_b = db::conn::init_pool(base_b.clone()).await.unwrap();
    backup::finish_restore_fixups(&db_b, &base_b).await.unwrap();

    let after = overview::totals(&db_b).await.unwrap();
    assert_eq!(
        (after.characters, after.conversations, after.messages, after.character_images, after.regex_scripts),
        (before.characters, before.conversations, before.messages, before.character_images, before.regex_scripts)
    );
    assert_eq!(std::fs::read_to_string(base_b.join("settings.json")).unwrap(), r#"{"app-setting":{"language":"en"}}"#);

    // 아바타 절대 경로가 이 PC 경로로 바뀜
    let (avatar,): (String,) = sqlx::query_as("SELECT avatar FROM characters").fetch_one(&db_b).await.unwrap();
    assert!(Path::new(&avatar).starts_with(&base_b), "{avatar}");
    assert!(Path::new(&avatar).is_file());
    // 채팅 이미지도 그대로
    let o = overview::data_overview(&db_b, &base_b).await.unwrap();
    let images = db::character_image::get_character_images(&db_b, &o.characters[0].id, false).await.unwrap();
    assert!(images.iter().all(|i| base_b.join(&i.file_path).is_file()));

    // 원래 있던 데이터는 지우지 않고 restore-backups로 옮겨 둠
    let kept: Vec<_> = std::fs::read_dir(base_b.join("restore-backups")).unwrap().collect();
    assert_eq!(kept.len(), 1);
    let kept = kept[0].as_ref().unwrap().path();
    assert!(kept.join("characters/old.txt").is_file());
    assert!(kept.join("app.db").is_file());
    assert!(!base_b.join("characters/old.txt").exists());
}

#[test]
fn schema_check_rejects_newer_or_modified_migrations() {
    let app: Vec<backup::MigrationInfo> = db::conn::schema_migrations()
        .into_iter()
        .map(|(version, c)| backup::MigrationInfo {
            version,
            checksum: c.iter().map(|b| format!("{b:02x}")).collect(),
        })
        .collect();
    assert!(backup::check_schema(&app).is_ok());
    assert!(backup::check_schema(&app[..1]).is_ok(), "예전 백업(마이그레이션 일부)은 허용");

    let mut modified = app.clone();
    modified[1].checksum = "00".into();
    let msg = backup::check_schema(&modified).unwrap_err().to_response().message;
    assert!(msg.contains("DB 구조가 다른 빌드"), "{msg}");

    let mut newer = app.clone();
    newer.push(backup::MigrationInfo { version: 99999, checksum: "aa".into() });
    assert!(backup::check_schema(&newer).is_err());
}

#[tokio::test]
async fn restore_rejects_path_traversal_and_cleans_staging() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("appdata");
    std::fs::create_dir_all(&base).unwrap();
    let bad = dir.path().join("bad.scbackup");
    {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&bad).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", opts).unwrap();
        zip.write_all(
            serde_json::to_string(&serde_json::json!({
                "format": backup::BACKUP_FORMAT, "version": 1, "appVersion": "x", "createdAt": 0,
                "schemaVersion": 0, "migrations": [], "appDataDir": "", "totals": {}
            }))
            .unwrap()
            .as_bytes(),
        )
        .unwrap();
        zip.start_file("app.db", opts).unwrap();
        zip.write_all(b"x").unwrap();
        zip.start_file("files/characters/../../evil.txt", opts).unwrap();
        zip.write_all(b"evil").unwrap();
        zip.finish().unwrap();
    }
    assert!(backup::prepare_restore(&base, &bad).await.is_err());
    assert!(!dir.path().join("evil.txt").exists());
    assert!(!base.join("restore-staging").exists(), "실패한 스테이징은 지움");
    assert!(!backup::apply_pending_restore(&base).unwrap());
}
