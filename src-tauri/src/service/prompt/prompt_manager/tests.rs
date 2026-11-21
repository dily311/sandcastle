use serde_json::json;

use super::*;
use crate::models::ai_service::HistoryItem;

fn ctx() -> ChatContext {
    ChatContext {
        user_images: vec![],
        image_prompt: None,
        image_keys: None,
        system_rules: vec!["Stay in character as {{char}}.".into()],
        char_name: Some("Aria".into()),
        user_name: Some("Min".into()),
        character_info: Some("{{char}} is a knight who protects {{user}}.".into()),
        persona_info: "A traveler.".into(),
        example_dialogue: None,
        main_prompt: None,
        world_info: vec![],
        history: vec![],
        extra_data: Value::Null,
    }
}

fn request(context: ChatContext, user_message: &str) -> PromptRequest {
    PromptRequest {
        context,
        user_message: user_message.into(),
        preset_id: None,
        context_limit: 0,
        world_info_ratio: 0.5,
        generation_type: GenerationType::Normal,
    }
}

fn hist(role: &str, content: &str) -> HistoryItem {
    HistoryItem {
        images: vec![],
        role: role.into(),
        content: content.into(),
    }
}

fn wi(id: &str, keys: &[&str], content: &str, constant: bool, priority: i64) -> WorldInfoEntry {
    WorldInfoEntry {
        comment: String::new(),
        id: id.into(),
        book_id: "b".into(),
        keys: keys.iter().map(|s| s.to_string()).collect(),
        secondary_keys: vec![],
        content: content.into(),
        enabled: true,
        priority,
        constant,
    }
}

fn shape(p: &PromptMessage) -> Vec<(PromptRole, String)> {
    p.messages.iter().map(|m| (m.role, m.content.clone())).collect()
}

fn block(identifier: &str, role: &str, content: &str, order: i64) -> PromptBlocks {
    PromptBlocks {
        id: format!("blk-{identifier}-{order}"),
        preset_id: "p".into(),
        name: identifier.into(),
        identifier: identifier.into(),
        block_type: "text".into(),
        role: role.into(),
        content: content.into(),
        enabled: true,
        position: "relative".into(),
        depth: 0,
        sort_order: order,
        triggers_json: json!([]),
        extra_json: None,
        created_at: 0,
        updated_at: 0,
    }
}

fn preset(blocks: Vec<PromptBlocks>, settings: Value) -> LoadedPreset {
    LoadedPreset {
        preset: PromptPresets {
            id: "p".into(),
            name: "test".into(),
            description: None,
            preset_type: "chat_completion".into(),
            source_format: "native".into(),
            config_json: json!({}),
            settings_json: Some(settings),
            compatibility_json: None,
            created_at: 0,
            updated_at: 0,
        },
        blocks,
    }
}

#[test]
fn default_layout_puts_rules_first_and_user_last() {
    let mut c = ctx();
    c.history = vec![hist("user", "hello"), hist("assistant", "greetings")];
    let p = PromptManager.build(request(c, "how are you?"), None).unwrap();
    let s = shape(&p);

    assert_eq!(s[0], (PromptRole::System, "Stay in character as Aria.".into()));
    assert_eq!(s[1], (PromptRole::System, "[Character: Aria]\nAria is a knight who protects Min.".into()));
    assert_eq!(s[2], (PromptRole::System, "[User: Min]\nA traveler.".into()));
    assert_eq!(s[3], (PromptRole::User, "hello".into()));
    assert_eq!(s[4], (PromptRole::Assistant, "greetings".into()));
    assert_eq!(s.last().unwrap(), &(PromptRole::User, "how are you?".into()));
    assert!(p.messages.iter().all(|m| m.token > 0));
}

#[test]
fn history_is_trimmed_from_oldest() {
    let mut c = ctx();
    c.history = (0..50).map(|i| hist("user", &format!("message number {i}"))).collect();
    let mut req = request(c.clone(), "latest");
    // 히스토리 없이 들어가는 고정 부분 + 메시지 3개 정도만 들어갈 예산
    let fixed = PromptManager
        .build(
            request(
                ChatContext {
                    user_images: vec![],
                    history: vec![],
                    ..c
                },
                "latest",
            ),
            None,
        )
        .unwrap()
        .total_tokens() as u32;
    let per_msg = count_message_tokens("message number 49") as u32;
    req.context_limit = fixed + per_msg * 3 + 1 + ChatOptions::default().max_tokens.unwrap();

    let p = PromptManager.build(req, None).unwrap();
    let s = shape(&p);
    let hist_msgs: Vec<_> = s.iter().filter(|(_, c)| c.starts_with("message number")).collect();
    assert_eq!(hist_msgs.len(), 3);
    assert_eq!(hist_msgs[0].1, "message number 47");
    assert_eq!(hist_msgs[2].1, "message number 49");
    assert_eq!(s.last().unwrap().1, "latest");
}

#[test]
fn world_info_uses_keywords_constants_and_priority() {
    let mut c = ctx();
    c.history = vec![hist("assistant", "The DRAGON roars.")];
    let mut secondary = wi("w4", &["castle"], "Castle needs secondary key.", false, 0);
    secondary.secondary_keys = vec!["gate".into()];
    let mut disabled = wi("w5", &[], "Disabled.", true, 0);
    disabled.enabled = false;
    c.world_info = vec![
        wi("w1", &["dragon"], "Dragons breathe fire.", false, 1),
        wi("w2", &[], "The kingdom is at war.", true, 0),
        wi("w3", &["elf"], "Elves are tall.", false, 9),
        secondary,
        disabled,
        wi("w1", &["dragon"], "Dragons breathe fire.", false, 1), // 중복 id
    ];
    let p = PromptManager.build(request(c, "we reach the castle"), None).unwrap();
    let wi_msg = p.messages.iter().find(|m| m.content.starts_with("[World Info]")).expect("월드인포 메시지");
    assert_eq!(wi_msg.content, "[World Info]\nThe kingdom is at war.\nDragons breathe fire.");
}

#[test]
fn world_info_respects_budget_ratio() {
    let mut c = ctx();
    c.world_info = vec![wi("big", &[], &"lore ".repeat(400), true, 10), wi("small", &[], "Short fact.", true, 0)];
    let mut req = request(c.clone(), "hi");
    let fixed = PromptManager
        .build(
            request(
                ChatContext {
                    user_images: vec![],
                    world_info: vec![],
                    ..c
                },
                "hi",
            ),
            None,
        )
        .unwrap()
        .total_tokens() as u32;
    req.context_limit = fixed + 100 + ChatOptions::default().max_tokens.unwrap();
    req.world_info_ratio = 0.5;

    let p = PromptManager.build(req, None).unwrap();
    let wi_msg = p.messages.iter().find(|m| m.content.starts_with("[World Info]")).unwrap();
    // 큰 항목은 예산(약 50토큰)을 넘어 빠지고 작은 항목만 남음
    assert_eq!(wi_msg.content, "[World Info]\nShort fact.");
}

#[test]
fn preset_blocks_markers_in_chat_and_triggers() {
    let mut c = ctx();
    c.history = vec![hist("user", "h1"), hist("assistant", "h2"), hist("user", "h3")];
    let mut in_chat_d1 = block("authorNote", "system", "Note for {{char}}", 10);
    in_chat_d1.position = "in_chat".into();
    in_chat_d1.depth = 1;
    let mut in_chat_d0 = block("jailbreak", "system", "Final instruction", 11);
    in_chat_d0.position = "in_chat".into();
    let mut only_continue = block("continueNudge", "system", "Continue the last reply", 3);
    only_continue.triggers_json = json!(["continue"]);
    let mut disabled = block("disabled", "system", "should not appear", 4);
    disabled.enabled = false;

    let pr = preset(
        vec![
            block("chatHistory", "user", "", 5),
            block("main", "system", "You are {{char}}.", 0),
            block("charDescription", "system", "", 1),
            only_continue,
            disabled,
            in_chat_d1,
            in_chat_d0,
            block("postHistory", "system", "Reply briefly.", 6),
        ],
        json!({ "temperature": 0.7, "openai_max_tokens": 300, "stream_openai": false, "top_k": 0 }),
    );

    let p = PromptManager.build(request(c, "now"), Some(&pr)).unwrap();
    let s: Vec<_> = shape(&p).into_iter().map(|(_, c)| c).collect();
    assert_eq!(
        s,
        vec![
            "Stay in character as Aria.",
            "You are Aria.",
            "Aria is a knight who protects Min.",
            "h1",
            "h2",
            "h3",
            "Note for Aria",
            "now",
            "Final instruction",
            "Reply briefly.",
        ]
    );
    assert_eq!(p.chat_options.temperature, Some(0.7));
    assert_eq!(p.chat_options.max_tokens, Some(300));
    assert!(!p.chat_options.stream);
    assert_eq!(p.chat_options.top_k, None);
}

#[test]
fn preset_without_history_marker_appends_history() {
    let pr = preset(vec![block("main", "system", "Main.", 0)], json!({}));
    let p = PromptManager.build(request(ctx(), "hi"), Some(&pr)).unwrap();
    let s = shape(&p);
    assert_eq!(s[1].1, "Main.");
    assert_eq!(s.last().unwrap(), &(PromptRole::User, "hi".into()));
}

#[test]
fn overflow_when_fixed_part_exceeds_budget() {
    let mut req = request(ctx(), "hi");
    req.context_limit = 1100; // 응답 몫 1024를 빼면 76토큰
    req.context.character_info = Some("long ".repeat(500));
    let err = PromptManager.build(req, None).unwrap_err();
    assert_eq!(err.to_response().code, ErrorCode::AiContextOverflow);
}

#[test]
fn macros_leave_unknown_and_unclosed_tags() {
    let m = Macros::from_context(&ctx());
    assert_eq!(m.apply("{{ CHAR }} & {{user}}"), "Aria & Min");
    assert_eq!(m.apply("{{unknown}} {{char"), "{{unknown}} {{char");
    assert_eq!(m.apply("a{{newline}}b{{trim}}"), "a\nb");
}

#[test]
fn preset_context_size_overrides_request_limit() {
    let mut req = request(ctx(), "hi");
    req.context_limit = 1_000_000;
    req.context.character_info = Some("long ".repeat(500));
    // 프리셋 컨텍스트 1100 - 응답 1024 = 76토큰 → 넘침
    let pr = preset(
        vec![block("main", "system", "Main.", 0), block("charDescription", "system", "", 1)],
        json!({ "context_size": 1100 }),
    );
    let err = PromptManager.build(req, Some(&pr)).unwrap_err();
    assert_eq!(err.to_response().code, ErrorCode::AiContextOverflow);
}

/// 캐릭터의 시스템 프롬프트 덮어쓰기는 프리셋의 main 블록 내용만 바꾼다
#[test]
fn character_main_prompt_replaces_main_block() {
    use crate::models::ai_service::{PromptBlocks, PromptPresets};
    let block = |id: &str, content: &str, order: i64| PromptBlocks {
        id: id.into(),
        preset_id: "p".into(),
        name: id.into(),
        identifier: id.into(),
        block_type: "text".into(),
        role: "system".into(),
        content: content.into(),
        enabled: true,
        position: "relative".into(),
        depth: 0,
        sort_order: order,
        triggers_json: serde_json::json!([]),
        extra_json: None,
        created_at: 0,
        updated_at: 0,
    };
    let preset = LoadedPreset {
        preset: PromptPresets {
            id: "p".into(),
            name: "p".into(),
            description: None,
            preset_type: "chat_completion".into(),
            source_format: "native".into(),
            config_json: serde_json::json!({}),
            settings_json: None,
            compatibility_json: None,
            created_at: 0,
            updated_at: 0,
        },
        blocks: vec![block("main", "기본 지시", 0), block("tail", "마무리", 1)],
    };
    let mut c = ctx();
    c.main_prompt = Some("{{char}} 전용 지시".into());
    let out = PromptManager.build(request(c, "안녕"), Some(&preset)).unwrap();
    let all: String = out.messages.iter().map(|m| m.content.clone()).collect::<Vec<_>>().join("\n");
    assert!(all.contains("전용 지시"), "{all}");
    assert!(!all.contains("기본 지시"), "{all}");
    assert!(all.contains("마무리"));
}

#[test]
fn attached_images_ride_along_with_their_messages() {
    let mut c = ctx();
    c.history = vec![
        HistoryItem {
            images: vec!["a.png".into()],
            role: "user".into(),
            content: "".into(),
        },
        hist("assistant", "nice picture"),
    ];
    c.user_images = vec!["b.jpg".into()];
    let p = PromptManager.build(request(c, "and this one?"), None).unwrap();
    let with_images: Vec<_> = p
        .messages
        .iter()
        .filter(|m| !m.images.is_empty())
        .map(|m| (m.content.as_str(), m.images.clone()))
        .collect();
    assert_eq!(with_images, vec![("", vec!["a.png".to_string()]), ("and this one?", vec!["b.jpg".to_string()])]);

    // 글 없이 이미지만 보내도 사용자 차례가 생김
    let mut c = ctx();
    c.user_images = vec!["only.png".into()];
    let p = PromptManager.build(request(c, ""), None).unwrap();
    assert_eq!(p.messages.last().map(|m| m.images.clone()), Some(vec!["only.png".to_string()]));
}

#[test]
fn world_info_ratio_reads_fraction_or_percent() {
    assert_eq!(world_info_ratio_from(&preset(vec![], json!({ "worldInfoRatio": 0.3 })).preset), Some(0.3));
    assert_eq!(world_info_ratio_from(&preset(vec![], json!({ "world_info_budget": 25 })).preset), Some(0.25));
    assert_eq!(world_info_ratio_from(&preset(vec![], json!({ "world_info_budget": 250 })).preset), Some(1.0));
    assert_eq!(world_info_ratio_from(&preset(vec![], json!({})).preset), None);
}

#[test]
fn reasoning_effort_comes_from_the_preset_and_auto_means_unset() {
    let with = |settings: Value| chat_options_from(&preset(vec![], settings).preset).reasoning_effort;
    assert_eq!(with(json!({ "reasoning_effort": "High" })).as_deref(), Some("high"));
    assert_eq!(with(json!({ "reasoning_effort": "auto" })), None);
    assert_eq!(with(json!({ "temperature": 1 })), None);
}

#[test]
fn image_rules_go_where_the_preset_puts_img_inprompt() {
    let mut c = ctx();
    c.image_prompt = Some("[이미지] {{char}}: 키 {{img_keys}}".into());
    c.image_keys = Some("smile, cry".into());
    // 자리를 정하지 않으면 시스템 규칙 끝에
    let plain = preset(
        vec![block("main", "system", "You are {{char}}.", 0), block("chatHistory", "user", "", 1)],
        json!({}),
    );
    let p = PromptManager.build(request(c.clone(), "hi"), Some(&plain)).unwrap();
    assert!(shape(&p)[0].1.ends_with("[이미지] Aria: 키 smile, cry"), "{:?}", shape(&p));
    // {{img_inprompt}}가 있으면 그 블록 안에서만 (사용자가 앞뒤에 덧붙인 말과 함께), {{img_keys}}도 프리셋에서 쓸 수 있다
    let placed = preset(
        vec![
            block("main", "system", "You are {{char}}.", 0),
            block("imgs", "system", "그림 규칙:\n{{img_inprompt}}\n키만 {{IMG_KEYS}} 중에서", 1),
            block("chatHistory", "user", "", 2),
        ],
        json!({}),
    );
    let p = PromptManager.build(request(c, "hi"), Some(&placed)).unwrap();
    let s: Vec<_> = shape(&p).into_iter().map(|(_, c)| c).collect();
    assert_eq!(s.iter().filter(|x| x.contains("[이미지]")).count(), 1);
    assert!(
        s.contains(&"그림 규칙:\n[이미지] Aria: 키 smile, cry\n키만 smile, cry 중에서".to_string()),
        "{s:?}"
    );
}
