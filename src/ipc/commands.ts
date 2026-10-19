//! Tauri Command Wrapper 함수 모음
//! 새 커맨드 혹은 리스너 등록시 Registry 등록을 먼저 할 것

import type { MessageMeta } from "$/lib/type/conv";
import { invokeCommand, listenCommand } from "./invoke";
import type { CommandRegistry, EventRegistry } from "./commandTypes";

export const settingCommands = {
	getAppSetting: () => invokeCommand("get_app_setting"),
	updateAppSetting: (args: CommandRegistry["update_app_setting"]["args"]) =>
		invokeCommand("update_app_setting", args),
	listen_appSettingsChanged: (cb: EventRegistry["app_setting::changed"]["payload"]) =>
		listenCommand("app_setting::changed", (e) => cb(e.payload)),
	getFeatureSetting: () => invokeCommand("get_feature_setting"),
	updateFeatureSetting: (args: CommandRegistry["update_feature_setting"]["args"]) =>
		invokeCommand("update_feature_setting", args),
	listen_featureChanged: (cb: EventRegistry["feature_setting::changed"]["payload"]) =>
		listenCommand("feature_setting::changed", (e) => cb(e.payload)),
};

export const themeCommands = {
	getThemeSettings: () => invokeCommand("get_theme_setting"),
	updateThemeSettings: (args: CommandRegistry["update_theme_setting"]["args"]) =>
		invokeCommand("update_theme_setting", args),
	listen_themeSettingsChanged: (cb: EventRegistry["theme_setting::changed"]["payload"]) =>
		listenCommand("theme_setting::changed", (e) => cb(e.payload)),
	getThemeList: () => invokeCommand("get_theme_list"),
	loadTheme: (args: CommandRegistry["load_theme_content"]["args"]) => invokeCommand("load_theme_content", args),
	saveTheme: (args: CommandRegistry["save_theme"]["args"]) => invokeCommand("save_theme", args),
	deleteTheme: (args: CommandRegistry["delete_theme"]["args"]) => invokeCommand("delete_theme", args),
};

export const windowCommands = {
	toggleIgnoreCursor: (args: CommandRegistry["toggle_ignore_cursor"]["args"]) =>
		invokeCommand("toggle_ignore_cursor", args),
	updateInteractiveRegions: (args: CommandRegistry["update_interactive_regions"]["args"]) =>
		invokeCommand("update_interactive_regions", args),
	removeInteractiveRegions: (args: CommandRegistry["remove_interactive_regions"]["args"]) =>
		invokeCommand("remove_interactive_regions", args),
};

export const widgetCommands = {
	// 전역
	openWidgetWindow: () => invokeCommand("open_widget"),
	closeWidgetWindow: () => invokeCommand("close_widget"),
	toggleWidgetWindow: () => invokeCommand("toggle_widget"),
	listen_widgetHidden: (cb: () => void) => listenCommand("widget:hidden", () => cb()),
	listen_widgetShown: (cb: () => void) => listenCommand("widget:shown", () => cb()),
	is_autostart_enabled: () => invokeCommand("is_autostart_enabled"),
	set_autostart: (args: CommandRegistry["set_autostart"]["args"]) => invokeCommand("set_autostart", args),

	// widget::music
	getWidgetMusicSettings: () => invokeCommand("get_widget_music_settings"),
	saveWidgetMusicSettings: (args: CommandRegistry["save_widget_music_settings"]["args"]) => invokeCommand("save_widget_music_settings", args),
	getSelectedFolderMusics: (args: CommandRegistry["get_select_folder_music_data"]["args"]) => invokeCommand("get_select_folder_music_data", args),
	getAllPlaylist: () => invokeCommand("get_all_playlist"),
	create_playlist: (args: CommandRegistry["create_playlist"]["args"]) => invokeCommand("create_playlist", args),
	update_playlist: (args: CommandRegistry["update_playlist"]["args"]) => invokeCommand("update_playlist", args),
	delete_playlist: (args: CommandRegistry["delete_playlist"]["args"]) => invokeCommand("delete_playlist", args),
	getPlaylistMusic: (args: CommandRegistry["get_playlist_entry"]["args"]) => invokeCommand("get_playlist_entry", args),
	appendPlaylistMusic: (args: CommandRegistry["append_playlist_entry"]["args"]) => invokeCommand("append_playlist_entry", args),
	removePlaylistMusic: (args: CommandRegistry["remove_playlist_entry"]["args"]) => invokeCommand("remove_playlist_entry", args),
	updateMusicMeta: (args: CommandRegistry["update_music_meta"]["args"]) => invokeCommand("update_music_meta", args),
	updateMusicOrder: (args: CommandRegistry["update_playlist_music_order"]["args"]) => invokeCommand("update_playlist_music_order", args),
	updateMusicAlbumArtPos: (args: CommandRegistry["update_music_album_art_pos"]["args"]) => invokeCommand("update_music_album_art_pos", args),
	// utils
	setAlbumCover: (args: CommandRegistry["set_album_art"]["args"]) => invokeCommand("set_album_art", args),
};

export const utilsCommands = {
	filterExistingPaths: (args: CommandRegistry["filter_existing_paths"]["args"]) =>
		invokeCommand("filter_existing_paths", args),
};

export const APICommands = {
	// api profile
	getAllApiProfile: () => invokeCommand("get_all_api_profile"),
	getApiProfile: (args: CommandRegistry["get_api_profile"]["args"]) => invokeCommand("get_api_profile", args),
	createApiProfile: (args: CommandRegistry["create_api_profile"]["args"]) =>
		invokeCommand("create_api_profile", args),
	updateApiProfile: (args: CommandRegistry["update_api_profile"]["args"]) =>
		invokeCommand("update_api_profile", args),
	deleteApiProfile: (args: CommandRegistry["delete_api_profile"]["args"]) =>
		invokeCommand("delete_api_profile", args),

	setApiKey: (args: CommandRegistry["set_api_key"]["args"]) => invokeCommand("set_api_key", args),
	deleteApiKey: (args: CommandRegistry["delete_api_key"]["args"]) => invokeCommand("delete_api_key", args),
	hasApiKey: (args: CommandRegistry["has_api_key"]["args"]) => invokeCommand("has_api_key", args),

	// api service
	getModels: (args: CommandRegistry["get_models"]["args"]) => invokeCommand("get_models", args),
	sendMessages: (args: CommandRegistry["send_messages"]["args"]) => invokeCommand("send_messages", args),
	testMessage: (args: CommandRegistry["test_message"]["args"]) => invokeCommand("test_message", args),
};

export const dbCommands = {
	// Character
	getCharacters: () => invokeCommand("get_all_characters"),
	createCharacter: (args: CommandRegistry["create_character"]["args"]) => invokeCommand("create_character", args),
	updateCharacter: (args: CommandRegistry["update_character"]["args"]) => invokeCommand("update_character", args),
	deleteCharacter: (args: CommandRegistry["delete_character"]["args"]) => invokeCommand("delete_character", args),

	// WorldInfo
	getWorldInfoBooks: () => invokeCommand("get_all_worldinfo_books"),
	createWorldInfoBook: (args: CommandRegistry["create_worldinfo_book"]["args"]) =>
		invokeCommand("create_worldinfo_book", args),
	deleteWorldInfoBook: (args: CommandRegistry["delete_worldinfo_book"]["args"]) =>
		invokeCommand("delete_worldinfo_book", args),
	getWorldInfoEntries: (args: CommandRegistry["get_worldinfo_entries"]["args"]) =>
		invokeCommand("get_worldinfo_entries", args),
	upsertWorldInfoEntry: (args: CommandRegistry["upsert_worldinfo_entry"]["args"]) =>
		invokeCommand("upsert_worldinfo_entry", args),
	deleteWorldInfoEntry: (args: CommandRegistry["delete_worldinfo_entry"]["args"]) =>
		invokeCommand("delete_worldinfo_entry", args),

	// Conversation
	getAllConversations: () => invokeCommand("get_all_conversations"),
	getConversation: (args: CommandRegistry["get_conversation"]["args"]) => invokeCommand("get_conversation", args),
	createConversation: (args: CommandRegistry["create_conversation"]["args"]) =>
		invokeCommand("create_conversation", args),
	touchConversation: (args: CommandRegistry["touch_conversation"]["args"]) =>
		invokeCommand("touch_conversation", args),
	deleteConversation: (args: CommandRegistry["delete_conversation"]["args"]) =>
		invokeCommand("delete_conversation", args),

	// Message
	getMessages: (conversationId: string, limit = 50, before?: number) =>
		invokeCommand("get_messages", { conversationId, limit, before }),
	appendMessage: (args: CommandRegistry["append_message"]["args"]) => invokeCommand("append_message", args),
	updateMessage: (args: CommandRegistry["update_message"]["args"]) => invokeCommand("update_message", args),
	updateMessageContent: (args: CommandRegistry["update_message_content"]["args"]) =>
		invokeCommand("update_message_content", args),
	updateMessageMeta: (id: string, meta: MessageMeta) => invokeCommand("update_message_meta", { id, meta }),
	updateMessageSwipeId: (args: CommandRegistry["update_message_swipe_id"]["args"]) =>
		invokeCommand("update_message_swipe_id", args),
	deleteMessage: (args: CommandRegistry["delete_message"]["args"]) => invokeCommand("delete_message", args),

	// persona
	getPersonas: () => invokeCommand("get_all_personas"),
	createPersona: (args: CommandRegistry["create_persona"]["args"]) => invokeCommand("create_persona", args),
	updatePersona: (args: CommandRegistry["update_persona"]["args"]) => invokeCommand("update_persona", args),
	deletePersona: (args: CommandRegistry["delete_persona"]["args"]) => invokeCommand("delete_persona", args),
	getPersonaUsage: (args: CommandRegistry["get_persona_usage"]["args"]) => invokeCommand("get_persona_usage", args),

	// Conversation Setting
	getConvSetting: () => invokeCommand("get_conv_settings"),
	setSystemDefaultPersona: (args: CommandRegistry["set_system_default_persona"]["args"]) =>
		invokeCommand("set_system_default_persona", args),
};

export const gameCommands = {
	// importGamePack: (args: CommandRegistry["import_game_pack"]["args"]) => invokeCommand("import_game_pack", args),
	// exportGamePack: (args: CommandRegistry["export_game_pack"]["args"]) => invokeCommand("export_game_pack", args),

	// listGames: () => invokeCommand("list_games"),
	// saveGameBundle: (args: CommandRegistry["save_game_bundle"]["args"]) => invokeCommand("save_game_bundle", args),
	// upsertGameNode: (args: CommandRegistry["upsert_game_node"]["args"]) => invokeCommand("upsert_game_node", args),
	// loadGameBundle: (args: CommandRegistry["get_game_bundle"]["args"]) => invokeCommand("get_game_bundle", args),
	// deleteGame: (gameId: string) => invokeCommand("delete_game", { gameId }),

	// // 세이브 데이터
	// listSaves: (args: CommandRegistry["list_game_saves"]["args"]) => invokeCommand("list_game_saves", args),
	// saveGame: (args: CommandRegistry["save_game"]["args"]) => invokeCommand("save_game", args),
	// loadSave: (args: CommandRegistry["load_game_save"]["args"]) => invokeCommand("load_game_save", args),
	// deleteSave: (args: CommandRegistry["delete_game_save"]["args"]) => invokeCommand("delete_game_save", args),

	// // AI 노드 에디터의 캐릭터 드롭다운용 (새로 추가 필요한 커맨드입니다)
	// listCharacters: () => invokeCommand("get_list_characters"),

	// // 슬롯별 캐릭터 conversation 매핑 (game_slot_conversations 테이블)
	// getSlotConversationId: (args: CommandRegistry["get_slot_conversationId"]["args"]) =>
	// 	invokeCommand("get_slot_conversationId", args),
	// linkSlotConversation: (args: CommandRegistry["link_slot_conversation"]["args"]) =>
	// 	invokeCommand("link_slot_conversation", args),
};

export const gameEngineCommands = {
	startNewGame: (args: CommandRegistry["start_new_game"]["args"]) => invokeCommand("start_new_game", args),
	continueGame: (args: CommandRegistry["continue_game"]["args"]) => invokeCommand("continue_game", args),
	dispatchGameCommand: (args: CommandRegistry["dispatch_game_command"]["args"]) =>
		invokeCommand("dispatch_game_command", args),
	save_write: (args: CommandRegistry["save_write"]["args"]) => invokeCommand("save_write", args),
	saveRead: (args: CommandRegistry["save_read"]["args"]) => invokeCommand("save_read", args),
	saveList: () => invokeCommand("save_list"),


};
