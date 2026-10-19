import type { Character, WorldInfoBook, WorldInfoEntry, Conversation, Message, Persona, MessageMeta, } from "$type/conv";
import type {ApiProfileInput, ApiProfile} from "$type/aiService";
import type { AppSettings, CssTheme, FeatureSetting, ThemeSetting } from "$/lib/type/settingTypes";
import type { Hitbox } from "$/lib/interaction/interactive-registry";
import type { GameCommand, GameResponse, GameSave, NpcReply, GamePlayerSessions, GameMessages, GameConversations, GameJournal, GameManifest } from "$type/game";
import type { LibraryRoot, LibraryRootRaw, MusicMeta, MusicMetaInput, Playlist, PlaylistInput, PlaylistItem, WidgetMusicSetting } from "$/lib/features/impl/widget/music/types";
import type { PackageData, SaveReport } from "$type/game";

export type CommandName = keyof CommandRegistry;
type CommandDef<A, R> = {args: A; response: R;};
export type CommandHandlers = {[K in CommandName]?: (args: CommandRegistry[K]["args"], ) => Promise<CommandRegistry[K]["response"]> | CommandRegistry[K]["response"]};

export interface CommandRegistry {
    // setting & theme
    "get_app_setting": CommandDef<undefined, AppSettings>,
    "update_app_setting": CommandDef<{ patch: Partial<AppSettings> }, AppSettings>,
    "get_feature_setting": CommandDef<undefined, FeatureSetting>,
    "update_feature_setting": CommandDef<{ id: string, enabled: boolean }, void>,
    "get_theme_setting": CommandDef<undefined, ThemeSetting>,
    "update_theme_setting": CommandDef<{ patch: Partial<ThemeSetting> }, AppSettings>,
    "get_theme_list": CommandDef<undefined, CssTheme[]>,
    "load_theme_content": CommandDef<{ filename: string }, string>,
    "save_theme": CommandDef<{ filename: string; content: Omit<CssTheme, "filename"> }, void>,
    "delete_theme": CommandDef<{ filename: string }, void>,
    // ai service
    "get_all_api_profile": CommandDef<undefined, ApiProfile[]>,
    "get_api_profile": CommandDef<{ id: string }, ApiProfile>,
    "create_api_profile": CommandDef<{ input: ApiProfileInput, apiKey: string }, ApiProfile>,
    "update_api_profile": CommandDef<{ id: string, input: ApiProfileInput, apiKey: string }, void>,
    "delete_api_profile": CommandDef<{ id: string }, void>,
    "set_api_key": CommandDef<{ profileId: string; api_key: string }, void>,
    "delete_api_key": CommandDef<{ profileId: string }, void>,
    "has_api_key": CommandDef<{ profileId: string }, boolean>,
    // chat
    "get_models": CommandDef<{ profileId: string }, string>,
    "send_messages": CommandDef<{ profileId: string; conversationId: string; message: string; }, string>,
    "test_message": CommandDef<{profileId: string}, {status: number; message: string}>,
    // conversation
    "get_all_characters": CommandDef<undefined, Character[]>,
    "create_character": CommandDef<{ input: Omit<Character, "id" | "createdAt" | "updatedAt" | "messageCount"> }, Character>,
    "update_character": CommandDef<{ id: string; input: Omit<Character, "id" | "createdAt" | "updatedAt"> }, void>,
    "delete_character": CommandDef<{ id: string }, void>,
    "get_all_worldinfo_books": CommandDef<undefined, WorldInfoBook[]>,
    "create_worldinfo_book": CommandDef<{ input: { name: string; description: string | null } }, WorldInfoBook>,
    "delete_worldinfo_book": CommandDef<{ id: string }, void>,
    "get_worldinfo_entries": CommandDef<{ bookId: string }, WorldInfoEntry>,
    "upsert_worldinfo_entry": CommandDef<{ input: Partial<WorldInfoEntry> & { bookId: string; content: string | null } }, WorldInfoEntry>,
    "delete_worldinfo_entry": CommandDef<{ id: string }, void>,
    "get_all_conversations": CommandDef<undefined, Conversation[]>,
    "get_conversation": CommandDef<{ id: string }, Conversation>,
    "create_conversation": CommandDef<{input: Omit<Conversation, "id" | "createdAt" | "updatedAt" | "messageCount" | "pinned">;}, Conversation>,
    "touch_conversation": CommandDef<{ id: string }, number>,
    "delete_conversation": CommandDef<{ id: string }, number>,
    "get_messages": CommandDef<{conversationId: string, limit: number, before?: number}, Message[]>,
    "append_message": CommandDef<{ input: Omit<Message, "id" | "createdAt"> }, Message>,
    "update_message": CommandDef<{ id: string; content: string; meta: MessageMeta | null }, void>,
    "update_message_content": CommandDef<{ id: string; content: string }, void>,
    "update_message_meta": CommandDef<{id: string, meta: MessageMeta}, void>,
    "update_message_swipe_id": CommandDef<{ id: string; swipeId: number }, void>,
    "delete_message": CommandDef<{ id: string }, void>,
    "get_all_personas": CommandDef<undefined, Persona[]>,
    "create_persona": CommandDef<{ input: Omit<Persona, "id" | "createdAt" | "updatedAt"> }, Persona>,
    "update_persona": CommandDef<{ id: string; input: Omit<Persona, "id" | "createdAt" | "updatedAt"> }, void>,
    "delete_persona": CommandDef<{ id: string }, void>,
    "get_persona_usage": CommandDef<{ id: string }, { characterCount: number; isSystemDefault: boolean }>,
    "get_conv_settings": CommandDef<undefined, {efaultPersonaId: string }>,
    "set_system_default_persona": CommandDef<{ id: string }, void>,

    // game_manager
    "create_game": CommandDef<{title: string, templateId: string}, string>,
    "get_package_data": CommandDef<{gameId: string}, PackageData>,
    "save_package": CommandDef<{gameId: string, data: PackageData}, SaveReport>,
    "delete_game": CommandDef<{gameId: string}, undefined>,
    "import_game": CommandDef<{zipPath: string}, string>,
    "export_game": CommandDef<{gameId: string, outputDir: string}, string>,
    // 게임 플레이
    "open_game": CommandDef<{gameId: string}, GameManifest>,
    // game_engine
    "start_new_game": CommandDef<undefined, GameResponse>,
    "list_sessions": CommandDef<undefined, GamePlayerSessions[]>,
    "continue_game": CommandDef<{session_id: string}, GameResponse>,
    "load_save": CommandDef<{session_id: string, slot: number}, GameResponse>,
    "read_journal": CommandDef<{session_id: string, from_seq?: number, to_seq: number}, GameJournal[]>,
    "read_conversations": CommandDef<{session_id: string, from_seq?: number, to_seq: number}, GameConversations[]>,
    "read_messages": CommandDef<{conversation_id: string, from_seq?: number, to_seq: number}, GameMessages[]>,
    "dispatch_game_command": CommandDef<{command: GameCommand}, GameResponse>,
    "save_write": CommandDef<{slot: number, data: any}, void>,
    "save_read": CommandDef<{slot: number}, any>,
    "save_list": CommandDef<undefined, GameSave[]>,
    "npc_say": CommandDef<{character_id: string, input: string}, NpcReply>,
    
    // widget
    "open_widget": CommandDef<undefined, void>,
    "close_widget": CommandDef<undefined, void>,
    "toggle_widget": CommandDef<undefined, void>,
    "is_autostart_enabled": CommandDef<undefined, boolean>,
    "set_autostart": CommandDef<{enabled: boolean}, void>,
    "toggle_ignore_cursor": CommandDef<{ appWindow: Window; isHoveringAny: boolean }, void>,
    "update_interactive_regions": CommandDef<{ upserts: (Hitbox & { id: string })[]; removed: string[] }, void>,
    "remove_interactive_regions": CommandDef<{ ids: string[] }, void>,
    
    // widget::music
    "get_widget_music_settings": CommandDef<undefined, WidgetMusicSetting>,
    "save_widget_music_settings": CommandDef<{patch: Partial<WidgetMusicSetting>}, undefined>,
    "get_select_folder_music_data": CommandDef<{ folderPath: string }, LibraryRootRaw>,
    "get_all_playlist": CommandDef<undefined, Playlist[]>,
    "create_playlist": CommandDef<{input: PlaylistInput}, Playlist>,
    "update_playlist": CommandDef<{id: string, input: PlaylistInput}, undefined>,
    "delete_playlist": CommandDef<{id: string}, undefined>,
    "get_playlist_entry": CommandDef<{playlistId: string}, PlaylistItem[]>,
    "append_playlist_entry": CommandDef<{playlistId: string, musicSrc: string, sortOrder?: number}, undefined>,
    "remove_playlist_entry": CommandDef<{playlistId: string, musicSrc: string}, undefined>,
    "update_music_meta": CommandDef<{input: MusicMetaInput}, undefined>,
    "update_playlist_music_order": CommandDef<{playlistId: string, musicSrc: string, sortOrder: number}, undefined>,
    "update_music_album_art_pos": CommandDef<{path: string, pos: unknown}, undefined>,
    
    // utils
    "set_album_art": CommandDef<{filePath: string, imgPath: string}, string>,
    "filter_existing_paths": CommandDef<{paths: string[]}, string>,
};

export type EventName = keyof EventRegistry;
export type EventDef<A, R> = {payload:A; response: R;};
export interface EventRegistry {
    "app_setting::changed": EventDef<(settings: AppSettings) => void, AppSettings>,
    "feature_setting::changed": EventDef<(changed: {id: string, enabled: boolean}) => void, {id: string, enabled: boolean}>,
    "theme_setting::changed": EventDef<(settings: ThemeSetting) => void, ThemeSetting>,
    "widget:hidden": EventDef<undefined, void>,
    "widget:shown": EventDef<undefined, void>,
}
