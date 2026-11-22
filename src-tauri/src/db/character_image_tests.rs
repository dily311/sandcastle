//! conversations / character_images DB 동작 확인
use sqlx::sqlite::SqlitePoolOptions;

use crate::{
    db::{character_image as ci, conversation as conv},
    models::{character_image::CharacterImageRow, conversation::ConversationInput},
};

async fn pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
    sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c1', 'Aria', 0, 0)")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

fn image(id: &str, key: &str) -> CharacterImageRow {
    CharacterImageRow {
        id: id.into(),
        character_id: "c1".into(),
        key: key.into(),
        file_path: format!("characters/c1/images/{id}.webp"),
        thumb_path: None,
        enabled: true,
        sort_order: 0,
        created_at: 0,
        updated_at: 0,
        folder: String::new(),
    }
}

#[tokio::test]
async fn get_conversation_reads_count_flag_and_survives_deleted_character() {
    let pool = pool().await;
    let created = conv::create_conversation(
        &pool,
        ConversationInput {
            title: "t".into(),
            character_id: "c1".into(),
            persona_id: "default".into(),
            world_info_ids: vec![],
            mode: "phone".into(),
            meta: None,
            images_enabled: None,
        },
    )
    .await
    .unwrap();
    assert!(created.images_enabled);

    // 예전에는 bare 테이블에서 읽어서 message_count 컬럼이 없다는 오류가 났다
    let c = conv::get_conversation(&pool, &created.id).await.unwrap();
    assert_eq!(c.mode, "phone");
    assert_eq!(c.message_count, 0);
    assert!(c.images_enabled);

    assert!(conv::set_conversation_images_enabled(&pool, &created.id, false).await.unwrap());
    assert!(!conv::get_conversation(&pool, &created.id).await.unwrap().images_enabled);

    // 캐릭터를 지우면 character_id가 NULL이 되어도 읽혀야 한다
    ci::insert_character_image(&pool, &image("i1", "Smile")).await.unwrap();
    assert!(conv::delete_character(&pool, "c1").await.unwrap());
    let c = conv::get_conversation(&pool, &created.id).await.unwrap();
    assert_eq!(c.character_id, "");
    assert_eq!(conv::get_all_conversations(&pool).await.unwrap().len(), 1);
    // 이미지 행은 CASCADE로 삭제
    assert!(ci::get_character_images(&pool, "c1", false).await.unwrap().is_empty());
}

#[tokio::test]
async fn image_keys_are_unique_per_character_ignoring_case() {
    let pool = pool().await;
    ci::insert_character_image(&pool, &image("i1", "Smile")).await.unwrap();
    assert!(ci::image_key_exists(&pool, "c1", "", "smile", None).await.unwrap());
    assert!(!ci::image_key_exists(&pool, "c1", "", "smile", Some("i1")).await.unwrap());
    assert!(ci::insert_character_image(&pool, &image("i2", "SMILE")).await.is_err());

    ci::insert_character_image(&pool, &image("i3", "cry")).await.unwrap();
    ci::update_character_image(&pool, "i3", "cry", false, 5, "").await.unwrap();
    let enabled = ci::get_character_images(&pool, "c1", true).await.unwrap();
    assert_eq!(enabled.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(), vec!["Smile"]);
    assert_eq!(ci::next_image_sort_order(&pool, "c1").await.unwrap(), 6);
}

#[tokio::test]
async fn same_key_is_allowed_in_different_version_folders() {
    let pool = pool().await;
    let mut a = image("v1s", "smile");
    a.folder = "v1".into();
    let mut b = image("v2s", "smile");
    b.folder = "v2".into();
    ci::insert_character_image(&pool, &a).await.unwrap();
    ci::insert_character_image(&pool, &b).await.unwrap();
    let mut dup = image("v1x", "SMILE");
    dup.folder = "v1".into();
    assert!(ci::insert_character_image(&pool, &dup).await.is_err(), "같은 판 안에서는 겹치면 안 됨");
    assert!(ci::image_key_exists(&pool, "c1", "v2", "smile", None).await.unwrap());
    assert!(!ci::image_key_exists(&pool, "c1", "v3", "smile", None).await.unwrap());
    let folders: Vec<String> = ci::get_character_images(&pool, "c1", false)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.folder)
        .collect();
    assert_eq!(folders, vec!["v1", "v2"]);

    // 캐릭터가 쓸 판 (characters 행이 있어야 한다)
    sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c1', 'A', 1, 1)")
        .execute(&pool)
        .await
        .ok();
    assert_eq!(ci::get_asset_version(&pool, "c1").await.unwrap(), None);
    assert!(ci::set_asset_version(&pool, "c1", Some("v2")).await.unwrap());
    assert_eq!(ci::get_asset_version(&pool, "c1").await.unwrap().as_deref(), Some("v2"));
    ci::set_asset_version(&pool, "c1", Some("")).await.unwrap();
    assert_eq!(ci::get_asset_version(&pool, "c1").await.unwrap(), None, "빈 글은 전체");
}

#[tokio::test]
async fn conversation_world_info_comes_from_conversation_character_and_persona() {
    use crate::models::conversation::{WorldInfoBookInput, WorldInfoEntryInput};
    let pool = pool().await;
    let mut books = vec![];
    for name in ["conv", "char", "persona", "unrelated"] {
        let b = conv::create_worldinfo_book(
            &pool,
            WorldInfoBookInput {
                id: None,
                name: name.into(),
                description: None,
            },
        )
        .await
        .unwrap();
        conv::upsert_worldinfo_entry(
            &pool,
            WorldInfoEntryInput {
                comment: String::new(),
                category: String::new(),
                id: None,
                book_id: b.id.clone(),
                keys: vec![],
                secondary_keys: vec![],
                content: name.into(),
                enabled: true,
                priority: 0,
                constant: true,
            },
        )
        .await
        .unwrap();
        books.push(b.id);
    }
    conv::create_character_worldinfo_ids(&pool, "c1", &books[1]).await.unwrap();
    conv::create_persona_worldinfo_ids(&pool, "default", &books[2]).await.unwrap();
    let c = conv::create_conversation(
        &pool,
        ConversationInput {
            title: "t".into(),
            character_id: "c1".into(),
            persona_id: "default".into(),
            world_info_ids: vec![books[0].clone()],
            mode: "chat".into(),
            meta: None,
            images_enabled: None,
        },
    )
    .await
    .unwrap();

    let mut got: Vec<String> = conv::get_all_conv_worldinfo_book_with_entries(&pool, &c.id, "c1", "default")
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.content)
        .collect();
    got.sort();
    assert_eq!(got, vec!["char", "conv", "persona"]);
}
