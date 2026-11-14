//! 대화 화면용 쿼리 (폴더·분기·메시지 번호·보낸 프롬프트)
use sqlx::sqlite::SqlitePoolOptions;

use crate::{
    db::{chat_ui as ui, conversation as conv},
    models::conversation::{ConversationInput, ConversationOptions, MessageInput},
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

async fn new_conv(pool: &sqlx::SqlitePool) -> String {
    conv::create_conversation(
        pool,
        ConversationInput {
            title: "본편".into(),
            character_id: "c1".into(),
            persona_id: "default".into(),
            world_info_ids: vec![],
            mode: "chat".into(),
            meta: None,
            images_enabled: None,
        },
    )
    .await
    .unwrap()
    .id
}

fn msg(conv: &str, role: &str, content: &str) -> MessageInput {
    MessageInput {
        conversation_id: conv.into(),
        seq: 0,
        role: role.into(),
        name: None,
        content: content.into(),
        character_id: None,
        meta: None,
    }
}

#[tokio::test]
async fn push_message_numbers_messages_in_order() {
    let pool = pool().await;
    let c = new_conv(&pool).await;
    let a = ui::push_message(&pool, msg(&c, "user", "안녕")).await.unwrap();
    let b = ui::push_message(&pool, msg(&c, "assistant", "반가워")).await.unwrap();
    assert_eq!((a.seq, b.seq), (1, 2));
    assert_eq!(ui::last_message(&pool, &c).await.unwrap().unwrap().id, b.id);
}

#[tokio::test]
async fn deleting_folder_keeps_conversations() {
    let pool = pool().await;
    let c = new_conv(&pool).await;
    let f = ui::create_folder(&pool, "c1", "본편").await.unwrap();
    let f2 = ui::create_folder(&pool, "c1", "외전").await.unwrap();
    assert_eq!((f.sort_order, f2.sort_order), (0, 1));
    ui::move_to_folder(&pool, &c, Some(&f.id)).await.unwrap();
    assert_eq!(conv::get_conversation(&pool, &c).await.unwrap().folder_id.as_deref(), Some(f.id.as_str()));

    assert!(ui::delete_folder(&pool, &f.id).await.unwrap());
    let after = conv::get_conversation(&pool, &c).await.unwrap();
    assert_eq!(after.folder_id, None);
    assert_eq!(ui::list_folders(&pool, "c1").await.unwrap().len(), 1);
}

#[tokio::test]
async fn branch_copies_messages_up_to_the_chosen_one() {
    let pool = pool().await;
    let c = new_conv(&pool).await;
    ui::set_pinned(&pool, &c, true).await.unwrap();
    ui::set_options(&pool, &c, &ConversationOptions { persona_id: "default".into(), prompt_preset_id: None, api_profile_id: None })
        .await
        .unwrap();
    let _m1 = ui::push_message(&pool, msg(&c, "user", "1")).await.unwrap();
    let m2 = ui::push_message(&pool, msg(&c, "assistant", "2")).await.unwrap();
    let _m3 = ui::push_message(&pool, msg(&c, "user", "3")).await.unwrap();

    let b = ui::copy_conversation(&pool, &c, "분기", Some(&m2.id)).await.unwrap();
    let copied = conv::get_messages(&pool, &b, 100, None).await.unwrap();
    assert_eq!(copied.iter().map(|m| m.content.as_str()).collect::<Vec<_>>(), ["1", "2"]);
    let bc = conv::get_conversation(&pool, &b).await.unwrap();
    assert_eq!(bc.title, "분기");
    assert!(!bc.pinned, "복사본은 고정하지 않음");
    // 다음 메시지 번호는 복사본 안에서 이어진다
    assert_eq!(ui::next_seq(&pool, &b).await.unwrap(), 3);

    let all = ui::copy_conversation(&pool, &c, "복제", None).await.unwrap();
    assert_eq!(conv::get_messages(&pool, &all, 100, None).await.unwrap().len(), 3);

    assert!(ui::copy_conversation(&pool, &c, "x", Some("없는 메시지")).await.is_err());
}

#[tokio::test]
async fn prompt_snapshot_is_replaced_and_removed_with_message() {
    let pool = pool().await;
    let c = new_conv(&pool).await;
    let m = ui::push_message(&pool, msg(&c, "assistant", "응답")).await.unwrap();
    ui::save_prompt(&pool, &m.id, &serde_json::json!({ "v": 1 })).await.unwrap();
    ui::save_prompt(&pool, &m.id, &serde_json::json!({ "v": 2 })).await.unwrap();
    assert_eq!(ui::get_prompt(&pool, &m.id).await.unwrap().unwrap()["v"], 2);
    conv::delete_message(&pool, &m.id).await.unwrap();
    assert!(ui::get_prompt(&pool, &m.id).await.unwrap().is_none());
}

#[tokio::test]
async fn message_content_and_meta_updates_work() {
    let pool = pool().await;
    let c = new_conv(&pool).await;
    let m = ui::push_message(&pool, msg(&c, "assistant", "a")).await.unwrap();
    assert!(conv::update_message_content(&pool, &m.id, "b").await.unwrap());
    assert!(conv::update_message_meta(&pool, &m.id, serde_json::json!({ "hidden": true })).await.unwrap());
    let got = ui::get_message(&pool, &m.id).await.unwrap();
    assert_eq!(got.content, "b");
    assert_eq!(got.meta.unwrap()["hidden"], true);
}

#[tokio::test]
async fn persona_world_info_and_avatar_are_saved() {
    let pool = pool().await;
    for b in ["b1", "b2"] {
        sqlx::query("INSERT INTO worldinfo_books (id, name, created_at, updated_at) VALUES (?, ?, 0, 0)")
            .bind(b)
            .bind(b)
            .execute(&pool)
            .await
            .unwrap();
    }
    let input = |books: Vec<&str>| crate::models::conversation::PersonaInput {
        name: "민".into(),
        avatar: None,
        description: "대학생".into(),
        world_info_ids: books.into_iter().map(String::from).collect(),
        tags: vec!["학생".into()],
        color: "#2f8f8a".into(),
        is_system_default: false,
    };

    let mut tx = pool.begin().await.unwrap();
    let p = conv::create_persona(&mut *tx, input(vec!["b1", "b2"])).await.unwrap();
    conv::replace_persona_worldinfo(&mut tx, &p.id, &p.world_info_ids).await.unwrap();
    tx.commit().await.unwrap();
    let mut got = conv::get_persona(&pool, &p.id).await.unwrap().world_info_ids;
    got.sort();
    assert_eq!(got, vec!["b1", "b2"]);

    // 바꾸면 빠진 책은 연결이 끊김
    let mut tx = pool.begin().await.unwrap();
    conv::update_persona(&mut *tx, &p.id, input(vec!["b2"])).await.unwrap();
    conv::replace_persona_worldinfo(&mut tx, &p.id, &["b2".to_string()]).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(conv::get_persona(&pool, &p.id).await.unwrap().world_info_ids, vec!["b2"]);

    conv::set_persona_avatar(&pool, &p.id, Some("C:/a/persona-x.png")).await.unwrap();
    assert_eq!(conv::get_persona(&pool, &p.id).await.unwrap().avatar.as_deref(), Some("C:/a/persona-x.png"));
    conv::set_persona_avatar(&pool, &p.id, None).await.unwrap();
    assert_eq!(conv::get_persona(&pool, &p.id).await.unwrap().avatar, None);
}

#[tokio::test]
async fn phone_contacts_are_per_persona() {
    use crate::db::phone;
    let pool = pool().await;
    sqlx::query("INSERT INTO personas (id, name, created_at, updated_at) VALUES ('p2', '하루', 0, 0)")
        .execute(&pool)
        .await
        .unwrap();

    phone::add_contact(&pool, "default", "c1").await.unwrap();
    // 두 번 넣어도 하나
    phone::add_contact(&pool, "default", "c1").await.unwrap();
    assert_eq!(phone::list_contacts(&pool, "default").await.unwrap().len(), 1);
    assert!(phone::list_contacts(&pool, "p2").await.unwrap().is_empty());

    assert!(phone::set_status(&pool, "default", "c1", "훈련 중").await.unwrap());
    assert_eq!(phone::list_contacts(&pool, "default").await.unwrap()[0].status, "훈련 중");

    // 캐릭터가 지워지면 연락처도 사라짐
    sqlx::query("DELETE FROM characters WHERE id = 'c1'").execute(&pool).await.unwrap();
    assert!(phone::list_contacts(&pool, "default").await.unwrap().is_empty());
    assert!(!phone::remove_contact(&pool, "default", "c1").await.unwrap());
}
