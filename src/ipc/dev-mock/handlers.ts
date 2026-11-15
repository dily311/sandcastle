// 브라우저 개발 모드용 가짜 백엔드. 응답 모양은 생성된 bindings.ts 타입을 따른다.
// 화면을 만들 때 필요한 커맨드를 여기에 추가한다.
import { chatHandlers } from "./chat";
import { settingsHandlers } from "./settings";
import { gameHandlers } from "./game";
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
	feature: { feature: { "chat.images": true, widget: true, "widget.music": true } } as FeatureSetting,
};

export const handlers: Record<string, Handler> = {
	// 실제로는 htmlview:// 프로토콜. 브라우저 개발 모드는 blob 주소로 같은 문서를 흉내 낸다
	register_html_view: ({ html }) => {
		const h = String(html);
		const doc = /<html|<!doctype/i.test(h) ? h : `<!doctype html><meta charset="utf-8"><body style="margin:0;padding:8px;font-family:system-ui">${h}</body>`;
		const report = `<script>(()=>{const s=()=>parent.postMessage({type:"sc-html-height",h:Math.ceil(document.body?document.body.getBoundingClientRect().bottom+parseFloat(getComputedStyle(document.body).marginBottom||"0"):document.documentElement.scrollHeight)},"*");new ResizeObserver(s).observe(document.documentElement);addEventListener("load",s);s();setTimeout(s,300);setTimeout(s,1000)})()<\/script>`;
		return URL.createObjectURL(new Blob([doc + report], { type: "text/html" }));
	},
	get_app_setting: () => db.appSetting,
	update_app_setting: ({ patch }) => (db.appSetting = { ...db.appSetting, ...(patch as object) }),
	get_theme_setting: () => db.themeSetting,
	update_theme_setting: ({ patch }) => (db.themeSetting = { ...db.themeSetting, ...(patch as object) }),
	get_feature_setting: () => db.feature,
	list_core_plugins: () =>
		(
			[
				["phone", "폰", "캐릭터와 메신저처럼 대화하는 폰 화면", true, false],
				["game", "게임 모드", "게임 만들기·가져오기와 플레이", true, false],
				["chat.images", "캐릭터 이미지", "캐릭터 이미지 에셋과 <cg> 태그", true, false],
				["html", "HTML 렌더링", "AI 답변의 html 코드 블록 실행", true, false],
				["regex", "정규식", "메시지 글자 찾아 바꾸기", true, false],
				["widget", "위젯", "바탕화면 위젯", false, true],
				["translate", "번역", "AI 답변·내 입력 번역", false, false],
				["tts", "음성 읽기", "AI 답변을 음성으로", false, false],
				["imagegen", "이미지 생성", "대화 장면을 이미지로", false, false],
			] as const
		).map(([id, name, description, def, desktopOnly]) => ({ id, name, description, defaultEnabled: def, desktopOnly, enabled: db.feature.feature[id] ?? def })),
	update_feature_setting: ({ id, enabled }) => {
		db.feature.feature[id as string] = enabled as boolean;
		return null;
	},
	get_recoverable_tasks: () => [],
	...chatHandlers,
	...settingsHandlers,
	...gameHandlers,
	// ---- 위젯 (브라우저에서 위젯 화면 확인용) ----
	get_widget_music_settings: () => ({ resize: false, isMiniPlayer: false, isImgEdit: false, isPlayListOpen: false, pos: null, selectedFolder: null, currentPlaylistId: "default", currentSongIndex: 0 }),
	save_widget_music_settings: () => null,
	get_all_playlist: () => [{ id: "default", title: "기본 재생 목록", description: null, coverImage: null, createdAt: 0, updatedAt: 0 }],
	get_playlist_entry: () => [],
	get_select_folder_music_data: () => ({ folders: {}, rootFiles: [] }),
	update_interactive_regions: () => null,
	remove_interactive_regions: () => null,
	set_widget_click_through: () => null,
	open_widget: () => null,
	close_widget: () => null,
	toggle_widget: () => null,
	"plugin:window|close": () => null,
	"plugin:window|set_ignore_cursor_events": () => null,
};
