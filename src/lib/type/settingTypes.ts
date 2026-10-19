export type Language = "ko" | "en";
export type ColorSchema = "light" | "dark" | "system";
export type SelectTheme = "default" | string;

export interface AppSettings {
	autoUpdate: boolean;
	language: Language;
	selectedPreset: string;
	sendOnEnter: boolean;
	showTimestamps: boolean;
	selectedApiProfile: string | null;
	selectedCharacterId: string | null;
	selectedPersonaId: string;
	lastPlayedGameId: string | null;
}

export interface ThemeSetting {
	mode: ColorSchema;
	selectTheme: SelectTheme;
	isFrameless: boolean;
	accentColor: string;
	fontSize: number;
}

export interface CssTheme {
	filename: string;
	name: string;
	author?: string;
	version: string;
	previewUrl?: string; // 이미지 URL 또는 Base64 DataURI
	cssContent?: string;
	variables?: { key: string; value: string };
}

export interface FeatureSetting {
	feature: Map<string, boolean>;
}
