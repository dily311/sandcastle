import { get_all_api_profile, get_all_characters, get_all_personas, get_app_setting, get_feature_setting, get_select_folder_music_data, get_theme_list, get_theme_setting, load_theme_content } from "$/data/mockup";
import type { CommandHandlers } from "./commandTypes";

// 각 커맨드별 모킹 핸들러 구현
export const mockHandlers: CommandHandlers = {
	get_app_setting: async (args) => get_app_setting,
	update_app_setting: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_feature_setting: async (args) => {
		throw new Error("연결되지 않음");
	},
	update_feature_setting: async (args) => undefined,
	get_theme_setting: async (args) => get_theme_setting,
	update_theme_setting: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_theme_list: async (args) => get_theme_list,
	load_theme_content: async (args) => load_theme_content,
	save_theme: async (args) => undefined,
	delete_theme: async (args) => undefined,
	toggle_ignore_cursor: async (args) => undefined,
	update_interactive_regions: async (args) => undefined,
	remove_interactive_regions: async (args) => undefined,
	open_widget: async (args) => undefined,
	close_widget: async (args) => undefined,
	toggle_widget: async (args) => undefined,
	is_autostart_enabled: async (args) => {
		throw new Error("연결되지 않음");
	},
	set_autostart: async (args) => undefined,
	get_select_folder_music_data: async (args) => {
		throw new Error("연결되지 않음");
	},
	set_album_art: async (args) => {
		throw new Error("연결되지 않음");
	},
	filter_existing_paths: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_all_api_profile: async (args) => get_all_api_profile,
	get_api_profile: async (args) => {
		throw new Error("연결되지 않음");
	},
	create_api_profile: async (args) => {
		throw new Error("연결되지 않음");
	},
	update_api_profile: async (args) => undefined,
	delete_api_profile: async (args) => undefined,
	set_api_key: async (args) => {
		throw new Error("연결되지 않음");
	},
	delete_api_key: async (args) => {
		throw new Error("연결되지 않음");
	},
	has_api_key: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_models: async (args) => {
		throw new Error("연결되지 않음");
	},
	send_messages: async (args) => {
		throw new Error("연결되지 않음");
	},
	test_message: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_all_characters: async (args) => get_all_characters,
	create_character: async (args) => {
		throw new Error("연결되지 않음");
	},
	update_character: async (args) => {
		throw new Error("연결되지 않음");
	},
	delete_character: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_all_worldinfo_books: async (args) => {
		throw new Error("연결되지 않음");
	},
	create_worldinfo_book: async (args) => {
		throw new Error("연결되지 않음");
	},
	delete_worldinfo_book: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_worldinfo_entries: async (args) => {
		throw new Error("연결되지 않음");
	},
	upsert_worldinfo_entry: async (args) => {
		throw new Error("연결되지 않음");
	},
	delete_worldinfo_entry: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_all_conversations: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_conversation: async (args) => {
		throw new Error("연결되지 않음");
	},
	create_conversation: async (args) => {
		throw new Error("연결되지 않음");
	},
	touch_conversation: async (args) => {
		throw new Error("연결되지 않음");
	},
	delete_conversation: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_messages: async (args) => {
		throw new Error("연결되지 않음");
	},
	append_message: async (args) => {
		throw new Error("연결되지 않음");
	},
	update_message: async (args) => {
		throw new Error("연결되지 않음");
	},
	update_message_content: async (args) => {
		throw new Error("연결되지 않음");
	},
	update_message_meta: async (args) => {
		throw new Error("연결되지 않음");
	},
	update_message_swipe_id: async (args) => {
		throw new Error("연결되지 않음");
	},
	delete_message: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_all_personas: async (args) => get_all_personas,
	create_persona: async (args) => {
		throw new Error("연결되지 않음");
	},
	update_persona: async (args) => {
		throw new Error("연결되지 않음");
	},
	delete_persona: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_persona_usage: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_conv_settings: async (args) => {
		throw new Error("연결되지 않음");
	},
	set_system_default_persona: async (args) => {
		throw new Error("연결되지 않음");
	},
	// import_game_pack: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// export_game_pack: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// list_games: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// save_game_bundle: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// upsert_game_node: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// get_game_bundle: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// delete_game: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// list_game_saves: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// save_game: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// load_game_save: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// delete_game_save: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// get_list_characters: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// get_slot_conversationId: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	// link_slot_conversation: async (args) => {
	// 	throw new Error("연결되지 않음");
	// },
	start_new_game: async (args) => {
		throw new Error("연결되지 않음");
	},
	continue_game: async (args) => {
		throw new Error("연결되지 않음");
	},
	dispatch_game_command: async (args) => {
		throw new Error("연결되지 않음");
	},
	save_write: async (args) => {
		throw new Error("연결되지 않음");
	},
	save_read: async (args) => {
		throw new Error("연결되지 않음");
	},
	save_list: async (args) => {
		return []
	},
	get_all_playlist: async (args) => {
		throw new Error("연결되지 않음");
	},
	get_playlist_entry: async (args) => {
		throw new Error("연결되지 않음");
	},
	list_sessions: async (args) => {
		throw new Error("연결되지 않음");
	},
	load_save: async (args) => {
		throw new Error("연결되지 않음");
	},
	npc_say: async (args) => {
		throw new Error("연결되지 않음");
	},
	read_conversations: async (args) => {
		throw new Error("연결되지 않음");
	},
	read_journal: async (args) => {
		throw new Error("연결되지 않음");
	},
	read_messages: async (args) => {
		throw new Error("연결되지 않음");
	},
};
