use std::{collections::HashMap, sync::Mutex};
use tauri::Manager;
pub mod state;
use state::LaunchMode;
mod error;
pub mod models;
mod adapter;
mod commands;
pub mod db;
pub mod game;
pub mod service;
pub mod sys;
use crate::{
    models::settings::{AppSetting, FeatureSetting, ThemeSetting},
    state::{AppState, WindowMode},
    sys::{settings::SettingsState, themes::ThemeManager},
};
use commands::{ai_service::*, conversation::*, game_engine::*, game_manager::*, settings::*};

#[cfg(desktop)]
mod widget;
#[cfg(desktop)]
use crate::widget::{manager::WidgetManager, MouseTrackerState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        // 플러그인 등록
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build());

    // 데스크탑에서만 실행될 기능 등록
    #[cfg(desktop)]
    {
        use crate::widget::MouseTrackerState;
        use commands::widget::*;
        use tauri_plugin_autostart::MacosLauncher;
        use tauri_plugin_autostart::ManagerExt;
        use widget::{WidgetInteractionState, InteractiveRegion, MusicWatcherState};

        builder = builder
            .plugin(tauri_plugin_window_state::Builder::default().build())
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_autostart::init(
                MacosLauncher::LaunchAgent,
                Some(vec!["--widget"]),
            ));
        builder = builder
            .manage(WidgetInteractionState::new())
            .manage(MouseTrackerState::new())
            .manage(MusicWatcherState::new());
        builder = builder.invoke_handler(tauri::generate_handler![
            // settings
            get_app_setting,
            update_app_setting,
            get_feature_setting,
            update_feature_setting,
            get_theme_setting,
            update_theme_setting,
            get_conv_settings,
            set_system_default_persona,
            // theme
            get_theme_list,
            load_theme_content,
            save_theme,
            delete_theme,
            // ai
            get_models,
            send_messages,
            stream_message,
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
            toggle_favorite_character,
            delete_character,
            get_list_characters,
            get_all_personas,
            create_persona,
            update_persona,
            delete_persona,
            get_persona_usage,
            get_all_worldinfo_books,
            create_worldinfo_book,
            delete_worldinfo_book,
            get_worldinfo_entries,
            upsert_worldinfo_entry,
            delete_worldinfo_entry,
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
            delete_sessions,
            dispatch_command,
            load_save,
            read_journal,
            read_conversations,
            read_messages,
            save_write,
            save_read,
            save_list,
            npc_say,
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
        ]);
    }
    // 모바일에서 실행될 기능 등록
    #[cfg(mobile)]
    {
        builder = builder.invoke_handler(tauri::generate_handler![
            // settings
            get_app_setting,
            update_app_setting,
            get_feature_setting,
            update_feature_setting,
            get_theme_setting,
            update_theme_setting,
            get_conv_settings,
            set_system_default_persona,
            // theme
            get_theme_list,
            load_theme_content,
            save_theme,
            delete_theme,
            // ai
            get_models,
            send_messages,
            stream_message,
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
            toggle_favorite_character,
            delete_character,
            get_list_characters,
            get_all_personas,
            create_persona,
            update_persona,
            delete_persona,
            get_persona_usage,
            get_all_worldinfo_books,
            create_worldinfo_book,
            delete_worldinfo_book,
            get_worldinfo_entries,
            upsert_worldinfo_entry,
            delete_worldinfo_entry,
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
            delete_sessions,
            dispatch_command,
            load_save,
            read_journal,
            read_conversations,
            read_messages,
            save_write,
            save_read,
            save_list,
            npc_say,
        ]);
    }

    // setup
    builder = builder.setup(move |app| {
        // 1. DB 초기화용 핸들 및 비동기 처리
        let db_handle = app.handle().clone();
        let pool = tauri::async_runtime::block_on(async move {
            let app_data_dir = db_handle
                .path()
                .app_data_dir()
                .expect("app_data_dir 조회 실패");
            db::conn::init_pool(app_data_dir)
                .await
                .expect("DB 초기화 실패")
        });

        // 2. AppState 초기화
        let launch_mode = LaunchMode::from_args();
        app.manage(AppState {
            db: pool,
            http_client: reqwest::Client::new(),
            active_archive: Mutex::new(None),
            active_runtime: tokio::sync::Mutex::new(None),
            ai_rate_limit: Mutex::new(HashMap::new()),
            window_mode: Mutex::new(WindowMode::Explore),
            launch_mode,
        });

        // 3. 앱 설정 로드
        let settings_handle = app.handle().clone();
        app.manage(SettingsState(Mutex::new(
            sys::settings::load::<AppSetting>(&settings_handle),
        )));
        app.manage(SettingsState(Mutex::new(
            sys::settings::load::<ThemeSetting>(&settings_handle),
        )));
        app.manage(SettingsState(Mutex::new(sys::settings::load::<
            FeatureSetting,
        >(&settings_handle))));

        // 4. 테마 매니저 초기화
        let app_dir = app.path().app_data_dir().expect("App data dir not found");
        let theme_manager = ThemeManager::new(app_dir);

        if let Err(err) = theme_manager.init_default_themes() {
            eprintln!("Failed to initialize themes: {}", err);
        }

        app.manage(theme_manager);

        #[cfg(desktop)]
        {
            // 윈도우 초기 상태
            let main_window = app
                .get_webview_window("main")
                .expect("main window not found");
            match launch_mode {
                LaunchMode::Normal => {
                    main_window.show()?;
                    main_window.set_focus()?;
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

    // 실행
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
