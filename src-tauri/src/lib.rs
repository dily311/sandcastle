use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tauri::Manager;
pub mod state;
use state::LaunchMode;
mod adapter;
mod commands;
pub mod db;
pub mod error;
pub mod game;
pub mod models;
pub mod service;
use service::ai::AiService;
pub mod sys;
pub mod task;
use crate::{
    models::settings::{AppSetting, FeatureSetting, ThemeSetting},
    state::{AppState, WindowMode},
    sys::{settings::SettingsState, themes::ThemeManager},
};
use commands::{
    ai_service::*, character_image::*, chat::*, conversation::*, convert::*, data::*, game_engine::*, game_manager::*, import::*, phone::*, preset::*,
    regex::*, settings::*, task::*,
};
use task::{TaskEventEmitter, TaskManager};

#[cfg(desktop)]
mod widget;
#[cfg(desktop)]
use crate::widget::{manager::WidgetManager, MouseTrackerState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 프론트 바인딩(src/ipc/bindings.ts)은 Rust 커맨드/타입에서 생성된다 (디버그 빌드 실행 시 갱신)
    let specta = specta_builder();
    #[cfg(all(debug_assertions, desktop))]
    export_bindings(&specta, concat!(env!("CARGO_MANIFEST_DIR"), "/../src/ipc/bindings.ts")).expect("TypeScript 바인딩 생성 실패");

    let mut builder = tauri::Builder::default()
        // 플러그인 등록
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_background_service::init_with_service(|| service::AppBackgroundService::new()))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        // Android content:// 파일 열기·저장 (service/picked_file.rs)
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init());

    // 데스크탑에서만 실행될 기능 등록
    #[cfg(desktop)]
    {
        use crate::widget::MouseTrackerState;
        use tauri_plugin_autostart::MacosLauncher;
        use widget::{MusicWatcherState, WidgetInteractionState};

        builder = builder
            .plugin(tauri_plugin_window_state::Builder::default().build())
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--widget"])));
        builder = builder
            .manage(WidgetInteractionState::new())
            .manage(MouseTrackerState::new())
            .manage(MusicWatcherState::new());
        builder = builder.invoke_handler(specta.invoke_handler());
    }
    // 모바일에서 실행될 기능 등록
    #[cfg(mobile)]
    {
        builder = builder.invoke_handler(specta.invoke_handler());
    }

    // setup
    builder = builder.setup(move |app| {
        // 1. DB 초기화용 핸들 및 비동기 처리
        let db_handle = app.handle().clone();
        let pool = tauri::async_runtime::block_on(async move {
            let app_data_dir = db_handle.path().app_data_dir().expect("app_data_dir 조회 실패");
            // 준비된 백업 복원이 있으면 DB를 열기 전에 파일을 교체한다
            // panic을 logs/panic.log에 남긴다 (1-D-6, 밖으로 보내지 않음)
            sys::panic_log::install(&app_data_dir);
            match service::data::backup::apply_pending_restore(&app_data_dir) {
                Ok(true) => eprintln!("[data] 백업 복원을 적용했습니다"),
                Ok(false) => {}
                Err(e) => eprintln!("[data] 백업 복원 실패: {e}"),
            }
            let pool = db::conn::init_pool(app_data_dir.clone()).await.expect("DB 초기화 실패");
            if let Err(e) = service::data::backup::finish_restore_fixups(&pool, &app_data_dir).await {
                eprintln!("[data] 복원 후 경로 보정 실패: {e}");
            }
            pool
        });

        // 2. AppState 초기화
        let app_handle = app.handle().clone();
        sys::panic_log::attach(app_handle.clone());
        let launch_mode = LaunchMode::from_args();
        app.manage(AppState {
            db: pool.clone(),
            http_client: reqwest::Client::new(),
            active_archive: Mutex::new(None),
            active_runtime: tokio::sync::Mutex::new(None),
            ai_rate_limit: Mutex::new(HashMap::new()),
            window_mode: Mutex::new(WindowMode::Explore),
            launch_mode,
        });

        // 2-1. 전역 작업 관리자
        // - 각 Manager는 여기서 register_executor로 자기 Executor를 등록한다 (지금은 채팅 생성).
        // - 지난 실행에서 끝나지 못한 작업은 Interrupted로 표시해 두고, 프론트가
        //   get_recoverable_tasks로 받아 재개/포기를 묻는다.
        let task_manager = TaskManager::new(pool.clone(), Arc::new(TaskEventEmitter::new(app_handle.clone())));
        task_manager.register_executor(Arc::new(service::chat::chat_task::ChatTaskExecutor::new(app_handle.clone())));
        if let Err(e) = tauri::async_runtime::block_on(task_manager.recover_on_startup()) {
            eprintln!("[task] 작업 복구 실패: {e}");
        }
        app.manage(task_manager);

        // 2-2. AI 호출 서비스 (API 키·프로필·공급자 어댑터)
        app.manage(Arc::new(AiService::new(pool.clone(), reqwest::Client::new())));

        // 3. 앱 설정 로드
        let settings_handle = app.handle().clone();
        app.manage(SettingsState(Mutex::new(sys::settings::load::<AppSetting>(&settings_handle))));
        app.manage(SettingsState(Mutex::new(sys::settings::load::<ThemeSetting>(&settings_handle))));
        let features = sys::settings::load::<FeatureSetting>(&settings_handle);
        service::chat::regex_processor::set_enabled(sys::plugins::enabled_in(&features, sys::plugins::id::REGEX));
        app.manage(SettingsState(Mutex::new(features)));
        // 사용자가 보존 기간을 골랐을 때만: 지난 작업·사용량 상세를 일별 합계로 정리 (1-A-4, 요금 없음)
        if let Some(days) = app.state::<SettingsState<AppSetting>>().0.lock().ok().and_then(|s| s.usage_retention_days) {
            let db = pool.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = service::budget::compact(&db, Some(days)).await {
                    eprintln!("[usage] 기록 정리 실패: {e}");
                }
            });
        }
        let plugin_setting = sys::settings::load::<models::settings::PluginSetting>(&settings_handle);
        service::app_prompts::set(&plugin_setting.prompts);
        app.manage(SettingsState(Mutex::new(plugin_setting)));

        // 4. 테마 매니저 초기화
        let app_dir = app.path().app_data_dir().expect("App data dir not found");
        // 기본 테마는 앱(프론트)에 들어 있음. 이 폴더에는 사용자 테마만
        let theme_manager = ThemeManager::new(app_dir);

        app.manage(theme_manager);

        #[cfg(desktop)]
        {
            // 윈도우 초기 상태
            let main_window = app.get_webview_window("main").expect("main window not found");
            match launch_mode {
                LaunchMode::Normal => {
                    main_window.show()?;
                    main_window.set_focus()?;
                    // 위젯이 켜져 있으면 여기서 연다 (창은 백엔드만 연다)
                    let wanted = app
                        .try_state::<SettingsState<models::settings::FeatureSetting>>()
                        .and_then(|s| s.0.lock().ok().map(|f| f.widget_window_wanted()))
                        .unwrap_or(false);
                    if wanted {
                        let state = app.state::<MouseTrackerState>();
                        let _ = WidgetManager::open_widget(app.handle(), &state);
                    }
                }

                LaunchMode::Widget => {
                    main_window.destroy()?;
                    let state = app.state::<MouseTrackerState>();
                    let _ = WidgetManager::open_widget(app.handle(), &state);
                }
            }
        }
        Ok(())
    });

    // AI 답변의 HTML 코드 블록 (iframe, sys/html_view.rs)
    builder = builder.register_uri_scheme_protocol("htmlview", |_app, req| sys::html_view::handle(req));

    // game 에셋 서빙
    builder = builder.register_uri_scheme_protocol("game", |app, req| {
        let app_handle = app.app_handle();
        let app_dir = app_handle.path().app_data_dir().expect("App data dir not found");
        let games_root = app_dir.join("games");

        game::protocol::handle(&games_root, req)
    });

    // 실행
    builder.run(tauri::generate_context!()).expect("error while running tauri application")
}

/// 커맨드 이벤트 페이로드처럼 커맨드 시그니처에 안 나오지만 프론트가 알아야 하는 타입
fn with_event_types(b: tauri_specta::Builder<tauri::Wry>) -> tauri_specta::Builder<tauri::Wry> {
    b.typ::<task::TaskEvent>()
        .typ::<models::ai_service::DialogueDoneEvent>()
        .typ::<models::ai_service::StreamPayload>()
        .typ::<models::settings::FeatureSetting>()
        // 게임 오류의 details.reason (GAME_ACTION_BLOCKED)
        .typ::<error::BlockReason>()
        // 타임스탬프(밀리초)·개수는 2^53 안이라 number로 충분
        .dangerously_cast_bigints_to_number()
}

/// Rust 커맨드 목록 = 프론트 바인딩 목록. 커맨드를 추가하면 여기에도 넣는다.
#[cfg(desktop)]
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    use commands::widget::*;
    with_event_types(tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        // settings
        get_app_setting,
        update_app_setting,
        get_feature_setting,
        update_feature_setting,
        get_theme_setting,
        update_theme_setting,
        add_backgrounds,
        remove_background,
        get_conv_settings,
        set_system_default_persona,
        // theme
        get_theme_list,
        load_theme_content,
        load_theme,
        save_theme,
        delete_theme,
        // ai
        get_models,
        test_message,
        chat_send,
        chat_regenerate,
        chat_cancel,
        list_phone_contacts,
        add_phone_contact,
        remove_phone_contact,
        set_phone_contact_status,
        phone_peek,
        get_message_prompt,
        list_conversation_folders,
        create_conversation_folder,
        rename_conversation_folder,
        delete_conversation_folder,
        rename_conversation,
        set_conversation_pinned,
        move_conversation,
        set_conversation_options,
        duplicate_conversation,
        branch_conversation,
        patch_conversation_meta,
        delete_messages,
        store_dropped_file,
        context_usage,
        display_messages,
        get_game_session_options,
        set_game_session_options,
        game_context_usage,
        edit_game_message,
        regenerate_game_reply,
        export_conversation,
        // task
        get_task,
        list_active_tasks,
        get_recoverable_tasks,
        cancel_task,
        resume_task,
        dismiss_task,
        list_task_history,
        usage_summary,
        clear_usage,
        allow_budget_once,
        compact_usage,
        list_budget_status,
        upsert_budget_limit,
        take_budget_warnings,
        list_model_prices,
        upsert_model_price,
        delete_model_price,
        // regex
        list_regex_scripts,
        upsert_regex_script,
        delete_regex_script,
        preview_regex_script,
        apply_regex,
        // character images (chat/phone)
        list_character_images,
        add_character_images,
        add_character_images_zip,
        update_character_image,
        delete_character_image,
        set_conversation_images_enabled,
        // import
        import_sillytavern,
        detect_sillytavern,
        // data manager
        get_data_overview,
        check_data,
        get_panic_log,
        clear_panic_log,
        stage_picked_file,
        prepare_save_target,
        deliver_saved_file,
        delete_conversations,
        export_character_pack,
        import_character_pack,
        export_backup,
        prepare_restore,
        cancel_restore,
        get_pending_restore,
        restart_app,
        // prompt preset
        list_prompt_presets,
        get_prompt_preset,
        save_prompt_preset,
        delete_prompt_preset,
        // api_profile
        get_api_profile,
        get_all_api_profile,
        create_api_profile,
        update_api_profile,
        delete_api_profile,
        set_api_key,
        delete_api_key,
        has_api_key,
        // data
        get_all_characters,
        create_character,
        update_character,
        set_character_avatar,
        set_character_cover,
        set_character_cover_position,
        copy_character_to_persona,
        copy_persona_to_character,
        copy_character_to_game,
        copy_game_character_to_character,
        toggle_favorite_character,
        delete_character,
        get_list_characters,
        get_all_personas,
        create_persona,
        update_persona,
        delete_persona,
        set_persona_avatar,
        get_persona_usage,
        get_all_worldinfo_books,
        create_worldinfo_book,
        delete_worldinfo_book,
        get_worldinfo_entries,
        upsert_worldinfo_entry,
        delete_worldinfo_entry,
        update_worldinfo_book,
        worldinfo_entry_counts,
        set_conversation_worldinfo,
        export_worldinfo_book,
        export_character_card,
        get_all_conversations,
        get_conversation,
        create_conversation,
        touch_conversation,
        delete_conversation,
        get_messages,
        append_message,
        update_message,
        update_message_content,
        update_message_meta,
        update_message_swipe_id,
        delete_message,
        // game_manager
        create_game,
        open_game,
        close_game,
        game_list,
        set_game_cover,
        list_templates,
        get_package_data,
        save_package,
        delete_game,
        import_game,
        export_game,
        // game_engine
        start_new_game,
        get_game_view,
        validate_game,
        continue_game,
        list_sessions,
        read_transcript,
        delete_sessions,
        dispatch_command,
        load_save,
        read_journal,
        sys::html_view::register_html_view,
        sys::plugins::list_core_plugins,
        commands::plugins::get_plugin_setting,
        commands::plugins::update_plugin_setting,
        commands::plugins::translate_text,
        commands::plugins::translate_message,
        commands::plugins::get_cached_translations,
        commands::plugins::speak_text,
        commands::plugins::generate_message_image,
        commands::plugins::generate_auto_image,
        commands::plugins::list_tts_voices,
        commands::plugins::list_generated_images,
        commands::plugins::delete_generated_image,
        commands::plugins::import_gallery_images,
        commands::plugins::list_all_images,
        commands::plugins::set_image_tags,
        commands::plugins::delete_gallery_images,
        commands::plugins::rename_gallery_image,
        save_png_file,
        commands::plugins::default_app_prompts,
        commands::plugins::generate_image_from_prompt,
        read_current_transcript,
        list_turn_points,
        fork_at,
        read_conversations,
        read_messages,
        save_write,
        save_read,
        save_list,
        npc_say,
        scene_say,
        // widget
        open_widget,
        close_widget,
        toggle_widget,
        toggle_ignore_cursor,
        is_autostart_enabled,
        set_autostart,
        update_interactive_regions,
        remove_interactive_regions,
        set_widget_click_through,
        // widget::music
        get_select_folder_music_data,
        get_widget_music_settings,
        save_widget_music_settings,
        get_all_playlist,
        create_playlist,
        update_playlist,
        delete_playlist,
        get_playlist_entry,
        append_playlist_entry,
        remove_playlist_entry,
        update_music_meta,
        update_playlist_music_order,
        update_music_album_art_pos,
        set_album_art,
        filter_existing_paths,
    ]))
}

#[cfg(mobile)]
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    with_event_types(tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        // settings
        get_app_setting,
        update_app_setting,
        get_feature_setting,
        update_feature_setting,
        get_theme_setting,
        update_theme_setting,
        add_backgrounds,
        remove_background,
        get_conv_settings,
        set_system_default_persona,
        // theme
        get_theme_list,
        load_theme_content,
        load_theme,
        save_theme,
        delete_theme,
        // ai
        get_models,
        test_message,
        chat_send,
        chat_regenerate,
        chat_cancel,
        list_phone_contacts,
        add_phone_contact,
        remove_phone_contact,
        set_phone_contact_status,
        phone_peek,
        get_message_prompt,
        list_conversation_folders,
        create_conversation_folder,
        rename_conversation_folder,
        delete_conversation_folder,
        rename_conversation,
        set_conversation_pinned,
        move_conversation,
        set_conversation_options,
        duplicate_conversation,
        branch_conversation,
        patch_conversation_meta,
        delete_messages,
        store_dropped_file,
        context_usage,
        display_messages,
        get_game_session_options,
        set_game_session_options,
        game_context_usage,
        edit_game_message,
        regenerate_game_reply,
        export_conversation,
        // task
        get_task,
        list_active_tasks,
        get_recoverable_tasks,
        cancel_task,
        resume_task,
        dismiss_task,
        list_task_history,
        usage_summary,
        clear_usage,
        allow_budget_once,
        compact_usage,
        list_budget_status,
        upsert_budget_limit,
        take_budget_warnings,
        list_model_prices,
        upsert_model_price,
        delete_model_price,
        // regex
        list_regex_scripts,
        upsert_regex_script,
        delete_regex_script,
        preview_regex_script,
        apply_regex,
        // character images (chat/phone)
        list_character_images,
        add_character_images,
        add_character_images_zip,
        update_character_image,
        delete_character_image,
        set_conversation_images_enabled,
        // import
        import_sillytavern,
        detect_sillytavern,
        // data manager
        get_data_overview,
        check_data,
        get_panic_log,
        clear_panic_log,
        stage_picked_file,
        prepare_save_target,
        deliver_saved_file,
        delete_conversations,
        export_character_pack,
        import_character_pack,
        export_backup,
        prepare_restore,
        cancel_restore,
        get_pending_restore,
        restart_app,
        // prompt preset
        list_prompt_presets,
        get_prompt_preset,
        save_prompt_preset,
        delete_prompt_preset,
        // api_profile
        get_api_profile,
        get_all_api_profile,
        create_api_profile,
        update_api_profile,
        delete_api_profile,
        set_api_key,
        delete_api_key,
        has_api_key,
        // data
        get_all_characters,
        create_character,
        update_character,
        set_character_avatar,
        set_character_cover,
        set_character_cover_position,
        copy_character_to_persona,
        copy_persona_to_character,
        copy_character_to_game,
        copy_game_character_to_character,
        toggle_favorite_character,
        delete_character,
        get_list_characters,
        get_all_personas,
        create_persona,
        update_persona,
        delete_persona,
        set_persona_avatar,
        get_persona_usage,
        get_all_worldinfo_books,
        create_worldinfo_book,
        delete_worldinfo_book,
        get_worldinfo_entries,
        upsert_worldinfo_entry,
        delete_worldinfo_entry,
        update_worldinfo_book,
        worldinfo_entry_counts,
        set_conversation_worldinfo,
        export_worldinfo_book,
        export_character_card,
        get_all_conversations,
        get_conversation,
        create_conversation,
        touch_conversation,
        delete_conversation,
        get_messages,
        append_message,
        update_message,
        update_message_content,
        update_message_meta,
        update_message_swipe_id,
        delete_message,
        // game_manager
        create_game,
        open_game,
        close_game,
        game_list,
        set_game_cover,
        list_templates,
        get_package_data,
        save_package,
        delete_game,
        import_game,
        export_game,
        // game_engine
        start_new_game,
        get_game_view,
        validate_game,
        continue_game,
        list_sessions,
        read_transcript,
        delete_sessions,
        dispatch_command,
        load_save,
        read_journal,
        sys::html_view::register_html_view,
        sys::plugins::list_core_plugins,
        commands::plugins::get_plugin_setting,
        commands::plugins::update_plugin_setting,
        commands::plugins::translate_text,
        commands::plugins::translate_message,
        commands::plugins::get_cached_translations,
        commands::plugins::speak_text,
        commands::plugins::generate_message_image,
        commands::plugins::generate_auto_image,
        commands::plugins::list_tts_voices,
        commands::plugins::list_generated_images,
        commands::plugins::delete_generated_image,
        commands::plugins::import_gallery_images,
        commands::plugins::list_all_images,
        commands::plugins::set_image_tags,
        commands::plugins::delete_gallery_images,
        commands::plugins::rename_gallery_image,
        save_png_file,
        commands::plugins::default_app_prompts,
        commands::plugins::generate_image_from_prompt,
        read_current_transcript,
        list_turn_points,
        fork_at,
        read_conversations,
        read_messages,
        save_write,
        save_read,
        save_list,
        npc_say,
        scene_say,
    ]))
}

/// TypeScript 바인딩 파일 생성 (`cargo run --bin export-bindings`로도 갱신 가능)
pub fn export_bindings(builder: &tauri_specta::Builder<tauri::Wry>, path: &str) -> Result<(), String> {
    builder
        .export(
            specta_typescript::Typescript::default()
                .header("// 이 파일은 Rust 커맨드에서 자동 생성됩니다 (src-tauri/src/lib.rs specta_builder). 직접 고치지 마세요.\n/* eslint-disable */"),
            path,
        )
        .map_err(|e| e.to_string())
}
