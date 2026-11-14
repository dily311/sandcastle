// 브라우저 개발 모드용 가짜 백엔드. 응답 모양은 생성된 bindings.ts 타입을 따른다.
// 화면을 만들 때 필요한 커맨드를 여기에 추가한다.
import { chatHandlers } from "./chat";
import { settingsHandlers } from "./settings";
import type { AppSetting_Serialize, FeatureSetting, ThemeSetting } from "../bindings";

type Args = Record<string, unknown>;
type Handler = (args: Args) => unknown | Promise<unknown>;

const db = {
	appSetting: {
		autoUpdate: true,
		language: "ko",
		selectedPreset: "default",
		sendOnEnter: true,
		showTimestamps: false,
		selectedApiProfile: "gemini",
		selectedCharacterId: null,
		selectedPersonaId: "default",
		lastPlayedGameId: null,
		phoneModePrompt: "[폰 메신저 대화]",
	} satisfies AppSetting_Serialize as AppSetting_Serialize,
	themeSetting: {
		mode: "system",
		selectTheme: "default",
		isFrameless: true,
		accentColor: "seaglass",
		fontSize: 16,
		grayTone: "slate",
	} satisfies ThemeSetting as ThemeSetting,
	feature: { feature: { "chat.images": true } } as FeatureSetting,
};

export const handlers: Record<string, Handler> = {
	get_app_setting: () => db.appSetting,
	update_app_setting: ({ patch }) => (db.appSetting = { ...db.appSetting, ...(patch as object) }),
	get_theme_setting: () => db.themeSetting,
	update_theme_setting: ({ patch }) => (db.themeSetting = { ...db.themeSetting, ...(patch as object) }),
	get_feature_setting: () => db.feature,
	update_feature_setting: ({ id, enabled }) => {
		db.feature.feature[id as string] = enabled as boolean;
		return null;
	},
	get_recoverable_tasks: () => [],
	...chatHandlers,
	...settingsHandlers,
};
