use sandcastle_lib::{self, db::*, models::conversation::*, models::game::*};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

#[tokio::test]
async fn database_migration_works() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");

    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    sqlx::migrate!("src/db/migrations").run(&pool).await?;
    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='conversations')",
    )
    .fetch_one(&pool)
    .await?;

    assert!(exists.0);
    Ok(())
}

#[cfg(test)]
mod character_test {
    use sandcastle_lib::{self, db::conversation, models::conversation::*};
    use sqlx::sqlite::SqlitePool;

    #[sqlx::test(migrations = "src/db/migrations")]
    async fn creates_character(pool: SqlitePool) -> Result<(), sqlx::error::Error> {
        let input = CharacterInput {
            name: "test".into(),
            avatar: None,
            description: "설명".into(),
            alternate_descriptions: vec![],
            first_message: None,
            alternate_first_message: vec![],
            example_dialogue: None,
            system_prompt_override: None,
            world_info_ids: vec![],
            tags: vec![],
            color: None,
            category: None,
            default_persona_id: None,
            default_prompt_preset_id: None,
            favorite: false,
        };
        let char = conversation::create_character(&pool, input).await?;
        assert_eq!(char.name, "test");

        let exists = conversation::char_exists(&pool, &char.id).await?;
        assert!(exists);

        let delete = conversation::delete_character(&pool, &char.id).await?;
        assert!(delete);
        Ok(())
    }
}

#[cfg(test)]
mod game_asset_test {
    use sandcastle_lib::{self, models::game::*};
    use sqlx::sqlite::SqlitePool;
    // 겜 프로젝트 테이블 테스트
    #[sqlx::test(migrations = "src/db/migrations")]
    async fn test_game_project_crud(pool: SqlitePool) -> Result<(), sqlx::error::Error> {
        let project_input = GameProjectInput {
            id: Some("game_1".into()),
            title: "test_1".into(),
            description: None,
            template_id: "default".into(),
            version: "1.0".into(),
            api_version: "1.0".into(),
            cover_image: None,
            start_scene_key: None,
            start_map_id: None,
            start_location_id: None,
            world_info_ids: vec![],
            custom_css: None,
            settings_json: None,
            created_at: None,
            updated_at: None,
        };

        crate::sandcastle_lib::db::game::upsert_game_project(&pool, &project_input).await?;

        Ok(())
    }
    // 겜 프로젝트 테이블 테스트
    #[sqlx::test(migrations = "src/db/migrations")]
    async fn test_game_character_crud(pool: SqlitePool) -> Result<(), sqlx::error::Error> {
        let project_input = GameProjectInput {
            id: Some("game_1".into()),
            title: "test_1".into(),
            description: None,
            template_id: "default".into(),
            version: "1.0".into(),
            api_version: "1.0".into(),
            cover_image: None,
            start_scene_key: None,
            start_map_id: None,
            start_location_id: None,
            world_info_ids: vec![],
            custom_css: None,
            settings_json: None,
            created_at: None,
            updated_at: None,
        };

        crate::sandcastle_lib::db::game::upsert_game_project(&pool, &project_input).await?;

        let game_character_input = GameCharactersInput {
            id: Some("char_1".into()),
            game_id: "game_1".into(),
            character_id: None,
            name: "char_1".into(),
            avatar: None,
            description: None,
            first_message: None,
            example_dialogue: None,
            system_prompt_override: None,
            behavior_type: None,
            world_info_ids: vec![],
            tags: None,
            settings_json: serde_json::Value::default(),
        };

        crate::sandcastle_lib::db::game::upsert_game_character(
            &pool,
            "game_1".into(),
            game_character_input,
        )
        .await?;
        Ok(())
    }

    // 1. 공통 기저 테이블인 game_assets 테스트
    #[sqlx::test(migrations = "src/db/migrations")]
    async fn test_game_assets_crud(pool: SqlitePool) -> Result<(), sqlx::error::Error> {
        let project_input = GameProjectInput {
            id: Some("game_1".into()),
            title: "test_1".into(),
            description: None,
            template_id: "default".into(),
            version: "1.0".into(),
            api_version: "1.0".into(),
            cover_image: None,
            start_scene_key: None,
            start_map_id: None,
            start_location_id: None,
            world_info_ids: vec![],
            custom_css: None,
            settings_json: None,
            created_at: None,
            updated_at: None,
        };

        crate::sandcastle_lib::db::game::upsert_game_project(&pool, &project_input).await?;

        let input = GameAssetsInput {
            id: Some("asset_1".into()),
            game_id: "game_1".into(),
            key: "bgm_main".into(),
            name: "Main Theme".into(),
            category: "bgm".into(),
            kind: "audio".into(),
            file_path: "/assets/bgm/main.ogg".into(),
            settings_json: Some(
                serde_json::from_str::<serde_json::Value>(r#"{"volume": 1.0, "loop": true}"#)
                    .ok()
                    .unwrap_or_default(),
            )
            .into(),
        };

        // Upsert 테스트
        let asset_id =
            crate::sandcastle_lib::db::game::upsert_game_asset(&pool, input, "game_1").await?;
        assert_eq!(asset_id, "asset_1");

        // 단건 조회 테스트
        let asset = crate::sandcastle_lib::db::game::get_game_asset(&pool, &asset_id).await?;
        assert_eq!(asset.name, "Main Theme");
        assert_eq!(asset.category, "bgm");

        // 전체 조회 테스트
        let all_assets =
            crate::sandcastle_lib::db::game::get_all_game_assets(&pool, "game_1").await?;
        assert_eq!(all_assets.len(), 1);

        // 삭제 테스트
        let deleted = crate::sandcastle_lib::db::game::delete_game_asset(&pool, &asset_id).await?;
        assert!(deleted);

        Ok(())
    }

    // 2. 오디오 에셋 테스트
    #[sqlx::test(migrations = "src/db/migrations")]
    async fn test_audio_asset_operations(pool: SqlitePool) -> Result<(), sqlx::error::Error> {
        let project_input = GameProjectInput {
            id: Some("game_1".into()),
            title: "test_1".into(),
            description: None,
            template_id: "default".into(),
            version: "1.0".into(),
            api_version: "1.0".into(),
            cover_image: None,
            start_scene_key: None,
            start_map_id: None,
            start_location_id: None,
            world_info_ids: vec![],
            custom_css: None,
            settings_json: None,
            created_at: None,
            updated_at: None,
        };

        crate::sandcastle_lib::db::game::upsert_game_project(&pool, &project_input).await?;

        let base_input = GameAssetsInput {
            id: Some("audio_asset_1".into()),
            game_id: "game_1".into(),
            key: "voice_hero_01".into(),
            name: "Hero Voice 1".into(),
            category: "voice".into(),
            kind: "audio".into(),
            file_path: "/assets/voice/hero_01.wav".into(),
            settings_json: None,
        };
        crate::sandcastle_lib::db::game::upsert_game_asset(&pool, base_input.clone(), "game_1")
            .await?;

        // Audio Input 준비
        let audio_input = AudioAssetsInput {
            base: base_input,
            tag: Some("battle".into()),
            game_character_id: None, // 필요시 character 생성 후 연결
            transcript: Some("적이다!".into()),
        };

        crate::sandcastle_lib::db::game::upsert_audio_asset(&pool, audio_input, "audio_asset_1")
            .await?;

        // 조회 테스트
        let audio =
            crate::sandcastle_lib::db::game::get_audio_asset(&pool, "audio_asset_1").await?;
        assert_eq!(audio.transcript.unwrap(), "적이다!");

        let list = crate::sandcastle_lib::db::game::get_all_audio_assets(&pool, "game_1").await?;
        assert_eq!(list.len(), 1);

        Ok(())
    }

    // 3. 캐릭터 에셋 테스트
    #[sqlx::test(migrations = "src/db/migrations")]
    async fn test_character_asset_operations(pool: SqlitePool) -> Result<(), sqlx::error::Error> {
        let project_input = GameProjectInput {
            id: Some("game_1".into()),
            title: "test_1".into(),
            description: None,
            template_id: "default".into(),
            version: "1.0".into(),
            api_version: "1.0".into(),
            cover_image: None,
            start_scene_key: None,
            start_map_id: None,
            start_location_id: None,
            world_info_ids: vec![],
            custom_css: None,
            settings_json: None,
            created_at: None,
            updated_at: None,
        };

        crate::sandcastle_lib::db::game::upsert_game_project(&pool, &project_input).await?;

        let game_character_input = GameCharactersInput {
            id: Some("char_1".into()),
            game_id: "game_1".into(),
            character_id: None,
            name: "char_1".into(),
            avatar: None,
            description: None,
            first_message: None,
            example_dialogue: None,
            system_prompt_override: None,
            behavior_type: None,
            world_info_ids: vec![],
            tags: None,
            settings_json: serde_json::Value::default(),
        };

        crate::sandcastle_lib::db::game::upsert_game_character(
            &pool,
            "game_1".into(),
            game_character_input,
        )
        .await?;
        let base_input = GameAssetsInput {
            id: Some("asset_1".into()),
            game_id: "game_1".into(),
            key: "hero_stand".into(),
            name: "Hero Standing".into(),
            category: "character".into(),
            kind: "image".into(),
            file_path: "/assets/chars/hero.png".into(),
            settings_json: None,
        };
        let asset_id =
            crate::sandcastle_lib::db::game::upsert_game_asset(&pool, base_input.clone(), "game_1")
                .await?;

        let char_asset_input = CharacterAssetsInput {
            base: base_input,
            game_character_id: Some("char_1".into()),
            usage: "standing".into(),
            emotion: Some("smile".into()),
        };

        crate::sandcastle_lib::db::game::upsert_character_asset(&pool, char_asset_input, &asset_id)
            .await?;

        let asset =
            crate::sandcastle_lib::db::game::get_character_asset(&pool, &asset_id).await?;
        assert_eq!(asset.usage, "standing");
        assert_eq!(asset.emotion.unwrap(), "smile");

        Ok(())
    }

    // 4. CG 에셋 테스트
    #[sqlx::test(migrations = "src/db/migrations")]
    async fn test_cg_asset_operations(pool: SqlitePool) -> Result<(), sqlx::error::Error> {
        let project_input = GameProjectInput {
            id: Some("game_1".into()),
            title: "test_1".into(),
            description: None,
            template_id: "default".into(),
            version: "1.0".into(),
            api_version: "1.0".into(),
            cover_image: None,
            start_scene_key: None,
            start_map_id: None,
            start_location_id: None,
            world_info_ids: vec![],
            custom_css: None,
            settings_json: None,
            created_at: None,
            updated_at: None,
        };

        crate::sandcastle_lib::db::game::upsert_game_project(&pool, &project_input).await?;

        let game_character_input = GameCharactersInput {
            id: Some("char_1".into()),
            game_id: "game_1".into(),
            character_id: None,
            name: "char_1".into(),
            avatar: None,
            description: None,
            first_message: None,
            example_dialogue: None,
            system_prompt_override: None,
            behavior_type: None,
            world_info_ids: vec![],
            tags: None,
            settings_json: serde_json::Value::default(),
        };

        crate::sandcastle_lib::db::game::upsert_game_character(
            &pool,
            "game_1".into(),
            game_character_input,
        )
        .await?;

        let base_input = GameAssetsInput {
            id: Some("cg_asset_1".into()),
            game_id: "game_1".into(),
            key: "cg_sunset".into(),
            name: "Sunset CG".into(),
            category: "cg".into(),
            kind: "image".into(),
            file_path: "/assets/cg/sunset.png".into(),
            settings_json: None,
        };
        let asset_id = crate::sandcastle_lib::db::game::upsert_game_asset(&pool, base_input.clone(), "game_1")
            .await?;

        let cg_input = CgAssetsInput {
            base: base_input,
            game_character_id: Some("char_1".into()),
            prompt: Some("beautiful sunset, masterpiece".into()),
            negative_prompt: Some("ugly, blurry".into()),
            seed: Some(12345),
            generation_params: Some("steps: 30".into()),
            description: None,
            ai_usable: false,
        };

        crate::sandcastle_lib::db::game::upsert_cg_asset(&pool, cg_input, &asset_id).await?;

        let asset = crate::sandcastle_lib::db::game::get_cg_asset(&pool, &asset_id).await?;
        assert_eq!(asset.seed.unwrap(), 12345);

        Ok(())
    }

    // 5. 배경 에셋 테스트
    #[sqlx::test(migrations = "src/db/migrations")]
    async fn test_background_asset_operations(pool: SqlitePool) -> Result<(), sqlx::error::Error> {
        let project_input = GameProjectInput {
            id: Some("game_1".into()),
            title: "test_1".into(),
            description: None,
            template_id: "default".into(),
            version: "1.0".into(),
            api_version: "1.0".into(),
            cover_image: None,
            start_scene_key: None,
            start_map_id: None,
            start_location_id: None,
            world_info_ids: vec![],
            custom_css: None,
            settings_json: None,
            created_at: None,
            updated_at: None,
        };

        crate::sandcastle_lib::db::game::upsert_game_project(&pool, &project_input).await?;

        let base_input = GameAssetsInput {
            id: Some("bg_asset_1".into()),
            game_id: "game_1".into(),
            key: "bg_forest".into(),
            name: "Forest Background".into(),
            category: "background".into(),
            kind: "image".into(),
            file_path: "/assets/bg/forest.png".into(),
            settings_json: None,
        };
        let asset_id = crate::sandcastle_lib::db::game::upsert_game_asset(&pool, base_input.clone(), "game_1")
            .await?;

        let bg_input = BackgroundAssetsInput {
            base: base_input,
            location_id: None,
            time_id: None,
            weather_id: None,
        };

        crate::sandcastle_lib::db::game::upsert_background_assets(&pool, bg_input, &asset_id)
            .await?;

        let asset =
            crate::sandcastle_lib::db::game::get_background_asset(&pool, &asset_id).await?;
        assert_eq!(asset.base.id, "bg_asset_1");

        Ok(())
    }
}
