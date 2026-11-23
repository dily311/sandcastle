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
    let SillyTavernImport::Character { character, .. } = import_file(db, base, &sample("character/default_Seraphina.png"), None).await.unwrap() else {
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
    if !crate::test_fixtures::present() {
        return;
    }
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

    // 지우기는 휴지통으로 (2-A-3): 목록에서 빠진다
    super::trash::move_to_trash(&db, &base, super::trash::TrashKind::Conversation, "orphan")
        .await
        .unwrap();
    assert!(overview::data_overview(&db, &base).await.unwrap().orphan_conversations.is_empty());
}

#[tokio::test]
async fn character_pack_round_trip_merges_with_new_ids() {
    if !crate::test_fixtures::present() {
        return;
    }
    let (_a, base_a, db_a) = app_dir().await;
    let old_id = populate(&db_a, &base_a).await;
    let fit = crate::models::conversation::AvatarFit { x: 30.0, y: 20.0, zoom: 1.5 };
    db::conversation::set_avatar_fit(&db_a, "characters", &old_id, Some(fit)).await.unwrap();
    // 변수 정의 (5-F-4-3)도 팩에 실려 간다
    let hp = crate::models::conversation::VarDef {
        key: "hp".into(),
        description: "체력".into(),
        kind: crate::models::conversation::VarKind::Number,
        default_value: Some("100".into()),
        min: Some(0.0),
        max: Some(100.0),
        max_step: Some(10.0),
        ai_read: true,
        ai_write: true,
    };
    db::conversation::upsert_character_var_defs(&db_a, &old_id, std::slice::from_ref(&hp))
        .await
        .unwrap();
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
    assert_eq!(
        (c.name.as_str(), c.message_count, c.image_count, c.world_info_book_count),
        ("Seraphina", 12, 3, 1)
    );
    assert_eq!(c.conversations[0].hidden_count, 3, "숨김 표시도 그대로");

    // 참조가 새 id로 바뀌었는지
    let msgs = db::conversation::get_messages(&db_b, &c.conversations[0].id, 100, None).await.unwrap();
    assert_eq!(msgs[0].character_id.as_deref(), Some(new_id.as_str()));
    assert_eq!(msgs[0].meta.as_ref().unwrap()["sender"]["id"], new_id.as_str());
    let regex = db::regex::get_regex_scripts_for(&db_b, Some(&new_id), None).await.unwrap();
    assert_eq!(regex.len(), 1);
    let character = db::conversation::get_character(&db_b, &new_id).await.unwrap();
    assert_eq!(character.avatar_fit, Some(fit), "프로필 그림 확대·위치도 그대로 (5-E-17-3)");
    assert_eq!(
        db::conversation::get_character_var_defs(&db_b, &new_id).await.unwrap(),
        vec![hp],
        "변수 정의도 그대로 (5-F-4-3)"
    );
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
    assert!(
        !base_b.join("tmp").read_dir().map(|mut d| d.next().is_some()).unwrap_or(false),
        "임시 파일 정리"
    );
}

#[tokio::test]
async fn pack_rejects_conversations_of_other_characters() {
    if !crate::test_fixtures::present() {
        return;
    }
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
    if !crate::test_fixtures::present() {
        return;
    }
    let (_a, base_a, db_a) = app_dir().await;
    let cid = populate(&db_a, &base_a).await;
    // 갤러리 그림(+정보), 올린 배경, 테마 배경을 가리키는 설정, 커버 경로 (1-B-1)
    for (rel, body) in [("generated/g1.png", "png"), ("generated/g1.json", "{}"), ("backgrounds/b1.png", "png")] {
        let p = base_a.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    let bg_a = base_a.join("backgrounds/b1.png").to_string_lossy().to_string();
    let settings =
        |bg: &str| serde_json::json!({ "app-setting": { "language": "en" }, "theme-setting": { "backgroundImage": bg, "backgrounds": [bg] } }).to_string();
    std::fs::write(base_a.join("settings.json"), settings(&bg_a)).unwrap();
    let cover_a = base_a.join("generated/g1.png").to_string_lossy().to_string();
    sqlx::query("UPDATE characters SET cover = ? WHERE id = ?")
        .bind(&cover_a)
        .bind(&cid)
        .execute(&db_a)
        .await
        .unwrap();
    let before = overview::totals(&db_a).await.unwrap();
    let out = base_a.join("exports/full.scbackup");
    let summary = backup::export_backup(&db_a, &base_a, &out).await.unwrap();
    // db + settings + 이미지 3장(원본·썸네일) + 아바타 + 갤러리 2 + 배경 1
    assert_eq!(summary.file_count, 1 + 1 + 3 * 2 + 1 + 2 + 1);

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
        (
            after.characters,
            after.conversations,
            after.messages,
            after.character_images,
            after.regex_scripts
        ),
        (
            before.characters,
            before.conversations,
            before.messages,
            before.character_images,
            before.regex_scripts
        )
    );
    // 설정 안의 테마 배경 경로가 이 PC 경로로 (JSON 안의 역슬래시 모양도)
    let bg_b = base_b.join("backgrounds/b1.png").to_string_lossy().to_string();
    let restored: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(base_b.join("settings.json")).unwrap()).unwrap();
    let expected: serde_json::Value = serde_json::from_str(&settings(&bg_b)).unwrap();
    assert_eq!(restored, expected);
    assert!(Path::new(&bg_b).is_file());
    // 갤러리 그림 그대로
    assert!(base_b.join("generated/g1.png").is_file() && base_b.join("generated/g1.json").is_file());

    // 아바타·커버 절대 경로가 이 PC 경로로 바뀜
    let (avatar, cover): (String, String) = sqlx::query_as("SELECT avatar, cover FROM characters").fetch_one(&db_b).await.unwrap();
    assert!(Path::new(&avatar).starts_with(&base_b), "{avatar}");
    assert!(Path::new(&avatar).is_file());
    assert!(Path::new(&cover).starts_with(&base_b) && Path::new(&cover).is_file(), "{cover}");
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

#[tokio::test]
async fn check_reports_integrity_and_only_unreferenced_files() {
    if !crate::test_fixtures::present() {
        return;
    }
    let (_d, base, db) = app_dir().await;
    populate(&db, &base).await;
    let clean = super::check::check_data(&db, &base).await.unwrap();
    assert!(clean.ok && clean.problems.is_empty());
    assert_eq!(clean.orphan_count, 0, "{:?}", clean.orphans);
    assert!(clean.checked_files >= 7); // 이미지 3장(원본·썸네일) + 아바타

    std::fs::write(base.join("characters/avatars/stray.png"), "12345").unwrap();
    let r = super::check::check_data(&db, &base).await.unwrap();
    assert_eq!(r.orphan_count, 1);
    assert_eq!((r.orphans[0].path.as_str(), r.orphans[0].bytes), ("characters/avatars/stray.png", 5));
    assert!(base.join("characters/avatars/stray.png").is_file(), "지우지 않음");
}

/// v1 백업(manifest.dirs 없음)을 복원해도 v1에 없던 갤러리·배경은 지금 것을 그대로 둔다
#[tokio::test]
async fn v1_backup_restore_keeps_folders_it_did_not_contain() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("appdata");
    let stage = base.join("restore-staging");
    for rel in ["generated/mine.png", "backgrounds/mine.png", "characters/now.txt"] {
        let p = base.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "x").unwrap();
    }
    std::fs::create_dir_all(stage.join("characters")).unwrap();
    std::fs::write(stage.join("app.db"), "db").unwrap();
    std::fs::write(stage.join("characters/old.txt"), "old").unwrap();
    // v1 manifest: dirs 칸이 없음
    let v1 = serde_json::json!({
        "format": backup::BACKUP_FORMAT, "version": 1, "appVersion": "0.1.0", "createdAt": 0, "schemaVersion": 1,
        "migrations": [], "appDataDir": "C:\\old", "totals": serde_json::to_value(overview::DataTotals::default()).unwrap(),
    });
    std::fs::write(stage.join("READY.json"), v1.to_string()).unwrap();
    let m = backup::pending_restore(&base).unwrap();
    assert_eq!(m.included_dirs(), vec!["characters", "games", "themes"]);

    assert!(backup::apply_pending_restore(&base).unwrap());
    assert!(base.join("generated/mine.png").is_file());
    assert!(base.join("backgrounds/mine.png").is_file());
    assert!(base.join("characters/old.txt").is_file() && !base.join("characters/now.txt").exists());
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
    newer.push(backup::MigrationInfo {
        version: 99999,
        checksum: "aa".into(),
    });
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

/// 1-B-4 / 2-B-4: API 키는 keyring에만. 백업·캐릭터 팩·동기화 묶음·앱 데이터 폴더, 그리고 묶음을 받아 합친
/// 다른 기기의 앱 데이터 폴더 어디에도 키 값이 없어야 한다 (가짜 키를 넣고 산출물 전체 — zip은 각 항목을 풀어서 — 에서 찾는다)
#[tokio::test]
async fn keys_never_leak_into_backup_pack_sync_or_app_data() {
    if !crate::test_fixtures::present() {
        return;
    }
    use crate::{
        models::ai_service::ApiProfilesInput,
        service::secret_store::{api_key_name, SecretStore},
    };
    use std::io::Read;

    let (_d, base, db) = app_dir().await;
    let cid = populate(&db, &base).await;
    let short = "sk-LEAKCHECK-short-7f3a9c1e5b";
    // 조각으로 나눠 저장되는 긴 키 (Vertex 서비스 계정 JSON 같은 것)
    let long = format!("{{\"private_key\":\"LEAKCHECK-LONG-{}\"}}", "Zq9".repeat(900));
    for (name, key) in [("short", short.to_string()), ("long", long.clone())] {
        let p = db::ai_service::create_api_profile(
            &db,
            ApiProfilesInput {
                name: name.into(),
                provider: "openai".into(),
                format_type: "openai".into(),
                base_url: "https://example.invalid".into(),
                model: "m".into(),
            },
        )
        .await
        .unwrap();
        SecretStore::set(&api_key_name(&p.id), &key).unwrap();
        assert_eq!(SecretStore::get(&api_key_name(&p.id)).unwrap(), key, "가짜 키가 저장소에 들어감");
    }
    std::fs::write(base.join("settings.json"), r#"{"app-setting":{"language":"ko"}}"#).unwrap();

    let backup = base.join("exports/full.scbackup");
    backup::export_backup(&db, &base, &backup).await.unwrap();
    let pack_out = base.join("exports/c.scpack");
    pack::export_pack(&db, &base, std::slice::from_ref(&cid), None, &pack_out).await.unwrap();
    // 동기화 묶음 (파일 포함, 설정 전부를 넘겨 settings_scope가 고르게)
    let settings = {
        use crate::models::settings::{AppSetting, FeatureSetting, PluginSetting, SettingsDomain, ThemeSetting};
        let mut m = serde_json::Map::new();
        m.insert(AppSetting::STORE_KEY.into(), serde_json::to_value(AppSetting::default()).unwrap());
        m.insert(ThemeSetting::STORE_KEY.into(), serde_json::to_value(ThemeSetting::default()).unwrap());
        m.insert(FeatureSetting::STORE_KEY.into(), serde_json::to_value(FeatureSetting::default()).unwrap());
        m.insert(PluginSetting::STORE_KEY.into(), serde_json::to_value(PluginSetting::default()).unwrap());
        m
    };
    let sync_out = base.join("exports/all.scsync");
    let info = super::sync_exchange::export(&db, &base, &settings, "dev-a", true, &sync_out).await.unwrap();
    assert!(info.rows > 0 && info.blobs > 0, "묶음에 행과 파일이 들어감: {info:?}");
    // 다른 기기에서 합치기 (적용 전 사본 sync-backups/도 생긴다)
    let other = _d.path().join("other");
    let other_db = db::conn::init_pool(other.clone()).await.unwrap();
    let mut other_settings = serde_json::Map::new();
    let applied = super::sync_exchange::apply(&other_db, &other, &mut other_settings, "dev-b", &sync_out)
        .await
        .unwrap();
    assert!(applied.report.added > 0);
    let profiles: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_profiles WHERE base_url = 'https://example.invalid'")
        .fetch_one(&other_db)
        .await
        .unwrap();
    assert_eq!(profiles, 2, "API 프로필 행은 넘어간다 (키는 기기마다 다시 넣음)");
    other_db.close().await;

    // 찾을 조각: 짧은 키 전체, 긴 키의 앞·뒤·가운데 조각 (keyring 조각 단위로 잘려 들어가도 걸리게)
    let needles: Vec<String> = vec![
        short.to_string(),
        long[..40].to_string(),
        long[long.len() - 40..].to_string(),
        long[1200..1240].to_string(),
    ];
    let hit = |bytes: &[u8]| needles.iter().find(|n| bytes.windows(n.len()).any(|w| w == n.as_bytes())).cloned();
    // 찾기가 실제로 걸리는지 (긴 키는 1000자 조각으로 나뉘어도 가운데 조각으로 걸림)
    assert!(hit(format!("x{short}y").as_bytes()).is_some() && hit(&long.as_bytes()[1000..2000]).is_some());

    for zip_path in [&backup, &pack_out, &sync_out] {
        let raw = std::fs::read(zip_path).unwrap();
        assert_eq!(hit(&raw), None, "{} (압축된 그대로)", zip_path.display());
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(raw)).unwrap();
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).unwrap();
            let mut body = vec![];
            entry.read_to_end(&mut body).unwrap();
            assert_eq!(hit(&body), None, "{} 안의 {}", zip_path.display(), entry.name());
        }
    }
    // 앱 데이터 폴더의 모든 파일 (DB 파일·WAL 포함)
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out)
            } else {
                out.push(p)
            }
        }
    }
    let mut files = vec![];
    walk(&base, &mut files);
    for f in files.iter().filter(|f| !f.starts_with(base.join("exports"))) {
        assert_eq!(hit(&std::fs::read(f).unwrap()), None, "{}", f.display());
    }
    // 묶음을 받아 합친 기기 (DB·sync-backups·넣은 파일 전부)
    let mut files = vec![];
    walk(&other, &mut files);
    assert!(files.iter().any(|f| f.starts_with(other.join("sync-backups"))), "적용 전 사본도 살펴봄");
    for f in &files {
        assert_eq!(hit(&std::fs::read(f).unwrap()), None, "받은 기기의 {}", f.display());
    }
}

// ---------------------------------------------------------------------------
// 휴지통 (2-A-3)
// ---------------------------------------------------------------------------

async fn count(db: &DbPool, sql: &str) -> i64 {
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_string())).fetch_one(db).await.unwrap()
}

#[tokio::test]
async fn trash_character_round_trip() {
    if !crate::test_fixtures::present() {
        return;
    }
    use super::trash::{self, TrashKind};
    let (_d, base, db) = app_dir().await;
    let cid = populate(&db, &base).await;
    sqlx::query("INSERT INTO phone_links (owner_kind, owner_id, contact_kind, contact_id, created_at) VALUES ('persona', 'default', 'character', ?, 1)")
        .bind(&cid)
        .execute(&db)
        .await
        .unwrap();
    let images_dir = base.join("characters").join(&cid);
    assert!(images_dir.exists());
    let images = count(&db, &format!("SELECT COUNT(*) FROM character_images WHERE character_id = '{cid}'")).await;
    let convs = count(&db, &format!("SELECT COUNT(*) FROM conversations WHERE character_id = '{cid}'")).await;
    assert!(images == 3 && convs >= 1);

    let item = trash::move_to_trash(&db, &base, TrashKind::Character, &cid).await.unwrap();
    assert_eq!(item.label, "Seraphina");
    assert!(item.rows >= 5 && item.files == 1, "{item:?}");
    // 지운 것처럼 보인다: 행·연결·폴더가 사라지고, 대화는 남되 캐릭터 연결만 끊긴다 (FK SET NULL)
    assert_eq!(count(&db, &format!("SELECT COUNT(*) FROM characters WHERE id = '{cid}'")).await, 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM character_images").await, 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM phone_links").await, 0);
    assert_eq!(count(&db, &format!("SELECT COUNT(*) FROM conversations WHERE character_id = '{cid}'")).await, 0);
    assert!(!images_dir.exists());
    // 다른 기기에 전할 지운 기록
    assert_eq!(
        count(
            &db,
            &format!("SELECT COUNT(*) FROM sync_tombstones WHERE tbl = 'characters' AND key = '[\"{cid}\"]'")
        )
        .await,
        1
    );
    assert_eq!(trash::list(&db).await.unwrap().len(), 1);

    let before = chrono::Utc::now().timestamp_millis() - 1;
    trash::restore(&db, &base, &item.id).await.unwrap();
    assert_eq!(
        count(&db, &format!("SELECT COUNT(*) FROM character_images WHERE character_id = '{cid}'")).await,
        images
    );
    assert_eq!(
        count(&db, &format!("SELECT COUNT(*) FROM conversations WHERE character_id = '{cid}'")).await,
        convs,
        "대화 연결이 다시 이어져야 함"
    );
    assert_eq!(count(&db, "SELECT COUNT(*) FROM phone_links").await, 1);
    assert!(images_dir.exists() && std::fs::read_dir(&images_dir).unwrap().count() > 0);
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM sync_tombstones WHERE tbl = 'characters'").await,
        0,
        "되살리면 지운 기록도 지움"
    );
    let at = count(&db, &format!("SELECT updated_at FROM characters WHERE id = '{cid}'")).await;
    assert!(at >= before, "되살린 행은 지금 고친 것으로");
    assert!(trash::list(&db).await.unwrap().is_empty());
    assert!(!base.join("trash").join(&item.id).exists());
}

#[tokio::test]
async fn trash_conversation_keeps_messages_and_purge_is_final() {
    if !crate::test_fixtures::present() {
        return;
    }
    use super::trash::{self, TrashKind};
    let (_d, base, db) = app_dir().await;
    populate(&db, &base).await;
    let conv: String = sqlx::query_scalar("SELECT id FROM conversations LIMIT 1").fetch_one(&db).await.unwrap();
    let msgs = count(&db, &format!("SELECT COUNT(*) FROM messages WHERE conversation_id = '{conv}'")).await;
    assert!(msgs > 0);

    let item = trash::move_to_trash(&db, &base, TrashKind::Conversation, &conv).await.unwrap();
    assert_eq!(count(&db, "SELECT COUNT(*) FROM messages").await, 0);
    trash::restore(&db, &base, &item.id).await.unwrap();
    assert_eq!(
        count(&db, &format!("SELECT COUNT(*) FROM messages WHERE conversation_id = '{conv}'")).await,
        msgs
    );

    // 완전히 지우기: 휴지통에서도 사라지고 되살릴 수 없다
    let item = trash::move_to_trash(&db, &base, TrashKind::Conversation, &conv).await.unwrap();
    assert!(trash::purge(&db, &base, &item.id).await.unwrap());
    assert!(trash::restore(&db, &base, &item.id).await.is_err());
    assert_eq!(count(&db, "SELECT COUNT(*) FROM conversations").await, 0);
    // 없는 항목은 NotFound
    assert!(matches!(
        trash::move_to_trash(&db, &base, TrashKind::Conversation, "nope").await,
        Err(crate::error::AppError::NotFound { .. })
    ));
}

#[tokio::test]
async fn trash_restore_refuses_when_parent_is_gone_and_expires() {
    use super::trash::{self, TrashKind};
    let (_d, base, db) = app_dir().await;
    sqlx::query("INSERT INTO game_projects (id, title, template_id, created_at, updated_at) VALUES ('g1', '숲', 'dating-sim', 1, 1)")
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO game_player_sessions (id, game_id, head_seq, updated_at) VALUES ('s1', 'g1', 0, 1)")
        .execute(&db)
        .await
        .unwrap();
    let session = trash::move_to_trash(&db, &base, TrashKind::GameSession, "s1").await.unwrap();
    assert_eq!(session.label, "숲 플레이 기록");
    // 그 사이 게임을 휴지통에 넣고 완전히 지우면, 세션은 되살릴 수 없다 (이유를 알린다)
    let game = trash::move_to_trash(&db, &base, TrashKind::Game, "g1").await.unwrap();
    trash::purge(&db, &base, &game.id).await.unwrap();
    let err = trash::restore(&db, &base, &session.id).await.unwrap_err();
    assert!(err.to_string().contains("game_projects"), "{err}");
    assert_eq!(count(&db, "SELECT COUNT(*) FROM game_player_sessions").await, 0, "실패하면 아무것도 넣지 않음");
    assert_eq!(trash::list(&db).await.unwrap().len(), 1, "실패한 항목은 휴지통에 남음");

    // 보존 기간: None이면 그대로, 지난 것은 정리
    assert_eq!(trash::purge_expired(&db, &base, None).await.unwrap(), 0);
    sqlx::query("UPDATE trash_items SET deleted_at = 0").execute(&db).await.unwrap();
    assert_eq!(trash::purge_expired(&db, &base, Some(30)).await.unwrap(), 1);
    assert!(trash::list(&db).await.unwrap().is_empty());
}

/// 하나씩 빠르게 지울 때 "데이터베이스가 사용 중" (5-E-28): 겹친 휴지통 옮기기가 둘 다 읽은 뒤 쓰려 하면
/// 지연 트랜잭션은 SQLite가 기다리지 않고 BUSY로 거절한다. BEGIN IMMEDIATE면 앞 것이 끝날 때까지 기다린다
#[tokio::test]
async fn overlapping_trash_moves_wait_instead_of_failing_busy() {
    if !crate::test_fixtures::present() {
        return;
    }
    use super::trash::{self, TrashKind};
    let (_d, base, db) = app_dir().await;
    let mut ids = vec![];
    for _ in 0..6 {
        let SillyTavernImport::Character { character, .. } = import_file(&db, &base, &sample("character/default_Seraphina.png"), None).await.unwrap() else {
            panic!()
        };
        ids.push(character.id);
    }
    let results = futures::future::join_all(ids.iter().map(|id| trash::move_to_trash(&db, &base, TrashKind::Character, id))).await;
    for r in &results {
        assert!(r.is_ok(), "{r:?}");
    }
    assert_eq!(count(&db, "SELECT COUNT(*) FROM characters").await, 0);
}
