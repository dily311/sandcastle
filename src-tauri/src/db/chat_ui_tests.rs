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
    ui::set_options(
        &pool,
        &c,
        &ConversationOptions {
            persona_id: "default".into(),
            prompt_preset_id: None,
            api_profile_id: None,
            model: None,
        },
    )
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

// 대화의 모델(ADR-020): API가 같으면 유지, API를 바꾸면 NULL로 초기화, 분기·복제에 따라간다
#[tokio::test]
async fn conversation_model_follows_its_api_and_is_copied_on_branch() {
    let pool = pool().await;
    let c = new_conv(&pool).await;
    for id in ["a1", "a2"] {
        sqlx::query("INSERT INTO api_profiles (id, name, provider, format_type, base_url, model, created_at, updated_at) VALUES (?, ?, 'openai', 'openai', '', 'm', 0, 0)")
            .bind(id)
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    let opts = |api: Option<&str>, model: Option<&str>| ConversationOptions {
        persona_id: "default".into(),
        prompt_preset_id: None,
        api_profile_id: api.map(Into::into),
        model: model.map(Into::into),
    };
    let model_of = |id: String| {
        let pool = pool.clone();
        async move { conv::get_conversation(&pool, &id).await.unwrap().model }
    };
    assert_eq!(model_of(c.clone()).await, None, "시작할 때는 프로필의 기본 모델을 따라감");
    // API를 정하고 모델을 고름 (API가 처음 정해지는 저장이라 모델은 초기화되지 않게 한 번에 보낸다)
    ui::set_options(&pool, &c, &opts(Some("a1"), None)).await.unwrap();
    ui::set_options(&pool, &c, &opts(Some("a1"), Some(" my-model "))).await.unwrap();
    assert_eq!(model_of(c.clone()).await.as_deref(), Some("my-model"));
    // 같은 API로 다른 설정만 저장해도 모델은 그대로 보낸 값
    ui::set_options(&pool, &c, &opts(Some("a1"), Some("my-model"))).await.unwrap();
    assert_eq!(model_of(c.clone()).await.as_deref(), Some("my-model"));
    // 분기·복제는 모델도 가져감
    let b = ui::copy_conversation(&pool, &c, "복제", None).await.unwrap();
    assert_eq!(model_of(b).await.as_deref(), Some("my-model"));
    // API를 바꾸면 초기화 (보낸 모델이 있어도 다른 공급자의 이름일 수 있으므로)
    ui::set_options(&pool, &c, &opts(Some("a2"), Some("my-model"))).await.unwrap();
    assert_eq!(model_of(c.clone()).await, None);
    // 빈 이름은 NULL
    ui::set_options(&pool, &c, &opts(Some("a2"), Some("  "))).await.unwrap();
    assert_eq!(model_of(c).await, None);
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

    use phone::Who;
    let me = Who {
        kind: "persona",
        id: "default",
    };
    let c1 = Who { kind: "character", id: "c1" };

    phone::add_contact(&pool, me, c1).await.unwrap();
    // 두 번 넣어도 하나
    phone::add_contact(&pool, me, c1).await.unwrap();
    assert_eq!(phone::list_contacts(&pool, me).await.unwrap().len(), 1);
    assert!(phone::list_contacts(&pool, Who { kind: "persona", id: "p2" }).await.unwrap().is_empty());

    assert!(phone::set_status(&pool, me, c1, "훈련 중").await.unwrap());
    assert_eq!(phone::list_contacts(&pool, me).await.unwrap()[0].status, "훈련 중");

    // 캐릭터 폰: 주인 = 캐릭터, 연락처 = 페르소나
    let char_owner = Who { kind: "character", id: "c1" };
    phone::add_contact(&pool, char_owner, Who { kind: "persona", id: "p2" }).await.unwrap();
    assert_eq!(phone::list_contacts(&pool, char_owner).await.unwrap()[0].contact_kind, "persona");

    // 캐릭터가 지워지면 그 캐릭터가 주인인 폰과 연락처 모두 사라짐 (트리거)
    sqlx::query("DELETE FROM characters WHERE id = 'c1'").execute(&pool).await.unwrap();
    assert!(phone::list_contacts(&pool, me).await.unwrap().is_empty());
    assert!(phone::list_contacts(&pool, char_owner).await.unwrap().is_empty());
    assert!(!phone::remove_contact(&pool, me, c1).await.unwrap());
}

#[tokio::test]
async fn character_update_saves_notes_links_and_defaults() {
    let pool = pool().await;
    sqlx::query("INSERT INTO worldinfo_books (id, name, created_at, updated_at) VALUES ('b1', 'b1', 0, 0)")
        .execute(&pool)
        .await
        .unwrap();
    let input = crate::models::conversation::CharacterInput {
        name: "Aria".into(),
        avatar: None,
        description: "기사 지망생".into(),
        alternate_descriptions: vec![],
        first_message: Some("안녕!".into()),
        alternate_first_message: vec!["반가워".into()],
        example_dialogue: None,
        system_prompt_override: None,
        creator_notes: Some("<b>v2</b> 업데이트".into()),
        world_info_ids: vec!["b1".into()],
        tags: vec!["기사".into()],
        color: Some("#c94f78".into()),
        category: None,
        default_persona_id: Some("default".into()),
        default_prompt_preset_id: None,
        favorite: true,
    };
    let mut tx = pool.begin().await.unwrap();
    conv::update_character(&mut *tx, "c1", &input).await.unwrap();
    conv::replace_character_worldinfo(&mut tx, "c1", &input.world_info_ids).await.unwrap();
    tx.commit().await.unwrap();

    let c = conv::get_character(&pool, "c1").await.unwrap();
    assert_eq!(c.creator_notes.as_deref(), Some("<b>v2</b> 업데이트"));
    assert_eq!(c.world_info_ids, vec!["b1"]);
    assert_eq!(c.default_persona_id.as_deref(), Some("default"));
    assert_eq!(c.alternate_first_message, vec!["반가워"]);
    assert!(c.favorite);

    // 새로 만들 때도 기본 페르소나·제작자 메모가 저장됨
    let created = conv::create_character(&pool, input).await.unwrap();
    let got = conv::get_character(&pool, &created.id).await.unwrap();
    assert_eq!(got.default_persona_id.as_deref(), Some("default"));
    assert_eq!(got.creator_notes.as_deref(), Some("<b>v2</b> 업데이트"));
}

#[tokio::test]
async fn conversation_meta_patch_merges_and_removes() {
    let pool = pool().await;
    let id = new_conv(&pool).await;
    ui::patch_conversation_meta(
        &pool,
        &id,
        &serde_json::json!({ "branch": { "parentId": "p1", "seq": 3 }, "authorNote": "비 오는 밤" }),
    )
    .await
    .unwrap();
    ui::patch_conversation_meta(&pool, &id, &serde_json::json!({ "authorNote": null, "memory": "열쇠를 잃어버림" }))
        .await
        .unwrap();
    let meta = conv::get_conversation(&pool, &id).await.unwrap().meta.unwrap();
    assert_eq!(meta["branch"]["parentId"], "p1");
    assert_eq!(meta["memory"], "열쇠를 잃어버림");
    assert!(meta.get("authorNote").is_none());
}

/// 대화 옵션 고르는 순서 (ADR-028, 5-E-29-1): 대화(자물쇠) > 캐릭터 기본 > 시스템 기본(사이드바 전역)
#[tokio::test]
async fn conversation_persona_falls_back_to_character_then_global() {
    let pool = pool().await;
    for id in ["p-char", "p-mine", "p-global"] {
        sqlx::query("INSERT INTO personas (id, name, created_at, updated_at) VALUES (?, ?, 0, 0)")
            .bind(id)
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE conv_settings SET default_persona_id = 'p-global' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let id = new_conv(&pool).await;
    // 빈 페르소나로 저장 = 풀림 (FK라 NULL로 들어간다)
    let opts = |p: &str| ConversationOptions {
        persona_id: p.into(),
        prompt_preset_id: None,
        api_profile_id: None,
        model: None,
    };
    ui::set_options(&pool, &id, &opts("")).await.unwrap();
    let c = conv::get_conversation(&pool, &id).await.unwrap();
    assert_eq!(c.persona_id, "");
    assert_eq!(
        conv::resolve_conv_persona(&pool, &c.persona_id, &c.character_id).await,
        "p-global",
        "캐릭터 기본이 없으면 전역"
    );
    sqlx::query("UPDATE characters SET default_persona_id = 'p-char' WHERE id = 'c1'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        conv::resolve_conv_persona(&pool, &c.persona_id, &c.character_id).await,
        "p-char",
        "캐릭터 기본이 전역보다 앞"
    );
    ui::set_options(&pool, &id, &opts("p-mine")).await.unwrap();
    let c = conv::get_conversation(&pool, &id).await.unwrap();
    assert_eq!(
        conv::resolve_conv_persona(&pool, &c.persona_id, &c.character_id).await,
        "p-mine",
        "잠근 대화의 값이 맨 앞"
    );
    // 캐릭터가 지워진 대화는 전역
    assert_eq!(conv::resolve_conv_persona(&pool, "", "").await, "p-global");
}

/// 마이그레이션 008: 일반 대화의 페르소나가 지금의 기본과 같으면 비우고(풀림), 다르면 남긴다(잠김). 폰 대화는 그대로
#[tokio::test]
async fn migration_008_unlocks_only_personas_equal_to_the_default() {
    let pool = pool().await;
    for id in ["p-char", "p-other"] {
        sqlx::query("INSERT INTO personas (id, name, created_at, updated_at) VALUES (?, ?, 0, 0)")
            .bind(id)
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO characters (id, name, default_persona_id, created_at, updated_at) VALUES ('c2', 'Bea', 'p-char', 0, 0)")
        .execute(&pool)
        .await
        .unwrap();
    // (id, 캐릭터, 페르소나, 모드)
    let rows = [
        ("a", "c1", "default", "chat"),  // 캐릭터 기본 없음 → 전역(default)과 같음 → 비움
        ("b", "c2", "p-char", "chat"),   // 캐릭터 기본과 같음 → 비움
        ("c", "c2", "p-other", "chat"),  // 일부러 고름 → 남김
        ("d", "c1", "default", "phone"), // 폰 주인 → 그대로
    ];
    for (id, ch, p, mode) in rows {
        sqlx::query("INSERT INTO conversations (id, title, character_id, persona_id, mode, created_at, updated_at) VALUES (?, 't', ?, ?, ?, 0, 100)")
            .bind(id)
            .bind(ch)
            .bind(p)
            .bind(mode)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::raw_sql(include_str!("migrations/008_conversation_persona_follows_default.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let got: Vec<(String, Option<String>, i64)> =
        sqlx::query_as("SELECT id, persona_id, updated_at FROM conversations WHERE id IN ('a','b','c','d') ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        got.iter().map(|(i, p, _)| (i.as_str(), p.as_deref())).collect::<Vec<_>>(),
        vec![("a", None), ("b", None), ("c", Some("p-other")), ("d", Some("default"))]
    );
    // 최근 대화 순서가 바뀌지 않게 updated_at은 1만 올린다 (touch 트리거가 지금 시각으로 올리지 않음)
    assert_eq!(got[0].2, 101);
    assert_eq!(got[2].2, 100);
}
