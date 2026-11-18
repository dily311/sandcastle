// 브라우저 개발 모드용 가짜 백엔드. 응답 모양은 생성된 bindings.ts 타입을 따른다.
// 화면을 만들 때 필요한 커맨드를 여기에 추가한다.
import { chatHandlers } from "./chat";
import { settingsHandlers } from "./settings";
import { gameHandlers } from "./game";
import { withTrash } from "./trash";
import { syncHandlers } from "./sync";
import { communityHandlers } from "./community";
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
		usageRetentionDays: null,
		trashRetentionDays: 30,
		checkUpdatesOnStart: false,
		home: { startOnHome: false, greeting: "", blocks: ["continue", "quick", "favorites", "games"], css: "" },
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

const base: Record<string, Handler> = {
	// 실제로는 htmlview:// 프로토콜. 브라우저 개발 모드는 blob 주소로 같은 문서를 흉내 낸다
	register_html_view: ({ html, theme }) => {
		const h = String(html);
		const themeStyle = theme ? `<style id="sc-theme">${String(theme).replace(/</g, "")}</style>` : "";
		const doc = /<html|<!doctype/i.test(h) ? h : `<!doctype html><meta charset="utf-8">${themeStyle}<body style="margin:0;padding:8px">${h}</body>`;
		const report = `<script>(()=>{const s=()=>parent.postMessage({type:"sc-html-height",h:Math.ceil(document.body?document.body.getBoundingClientRect().bottom+parseFloat(getComputedStyle(document.body).marginBottom||"0"):document.documentElement.scrollHeight)},"*");new ResizeObserver(s).observe(document.documentElement);addEventListener("load",s);s();setTimeout(s,300);setTimeout(s,1000)})()<\/script>`;
		const helper = `<script>(()=>{const post=(a,t)=>parent.postMessage({type:"sc-action",action:a,text:String(t??"").slice(0,4000)},"*");window.sandcastle=Object.freeze({fill:t=>post("fill",t),send:t=>post("send",t),choose:t=>post("send",t)});addEventListener("click",e=>{const el=e.target&&e.target.closest?e.target.closest("[data-sc-send],[data-sc-choice],[data-sc-fill]"):null;if(!el)return;e.preventDefault();const s=el.getAttribute("data-sc-send")??el.getAttribute("data-sc-choice");if(s!==null)post("send",s||el.textContent.trim());else post("fill",el.getAttribute("data-sc-fill")||el.textContent.trim())},true)})();<\/script>`;
		return URL.createObjectURL(new Blob([helper + doc + report], { type: "text/html" }));
	},
	get_app_setting: () => db.appSetting,
	update_app_setting: ({ patch }) => (db.appSetting = { ...db.appSetting, ...(patch as object) }),
	get_theme_setting: () => db.themeSetting,
	update_theme_setting: ({ patch }) => (db.themeSetting = { ...db.themeSetting, ...(patch as object) }),
	add_backgrounds: ({ paths, select }) => {
		const list = [...((db.themeSetting as { backgrounds?: string[] }).backgrounds ?? []), ...(paths as string[])];
		return (db.themeSetting = { ...db.themeSetting, backgrounds: list, ...(select ? { backgroundImage: list.at(-1) ?? null } : {}) });
	},
	remove_background: ({ path }) => {
		const cur = db.themeSetting as { backgrounds?: string[]; backgroundImage?: string | null };
		return (db.themeSetting = { ...db.themeSetting, backgrounds: (cur.backgrounds ?? []).filter((p) => p !== path), backgroundImage: cur.backgroundImage === path ? null : (cur.backgroundImage ?? null) });
	},
	add_fonts: ({ paths, select }) => {
		const cur = db.themeSetting as { fonts?: { path: string; name: string }[] };
		const added = (paths as string[]).map((p) => ({ path: p, name: (p.split(/[\\/]/).pop() ?? "글꼴").replace(/\.[^.]+$/, "") }));
		const list = [...(cur.fonts ?? []), ...added];
		return (db.themeSetting = { ...db.themeSetting, fonts: list, ...(select ? { appFont: list.at(-1)?.path ?? null } : {}) });
	},
	remove_font: ({ path }) => {
		const cur = db.themeSetting as { fonts?: { path: string; name: string }[]; appFont?: string | null };
		return (db.themeSetting = { ...db.themeSetting, fonts: (cur.fonts ?? []).filter((f) => f.path !== path), appFont: cur.appFont === path ? null : (cur.appFont ?? null) });
	},
	get_feature_setting: () => db.feature,
	list_core_plugins: () =>
		(
			[
				["phone", "폰", "캐릭터와 메신저처럼 대화하는 폰 화면", true, false],
				["game", "게임 모드", "게임 만들기·가져오기와 플레이", true, false],
				["chat.images", "캐릭터 에셋", "캐릭터 에셋 에셋과 <cg> 태그", true, false],
				["html", "HTML 렌더링", "AI 답변의 html 코드 블록 실행", false, false],
				["regex", "정규식", "메시지 글자 찾아 바꾸기", true, false],
				["widget", "위젯", "바탕화면 위젯", false, true],
				["translate", "번역", "AI 답변·내 입력 번역", false, false],
				["tts", "음성 읽기", "AI 답변을 음성으로", false, false],
				["imagegen", "이미지 생성", "직접 그림 만들기", false, false],
				["imageauto", "이미지 자동 생성", "글쓰기 AI가 그림 프롬프트도 쓰고 응답 뒤 자동으로 그림", false, false],
				["sync", "동기화", "다른 기기와 파일로 주고받아 합치기. API 키는 넘어가지 않음", false, false],
				["lint", "메시지 linter", "AI 응답이 저장될 때 닫히지 않은 태그·따옴표 등을 검사", false, false],
				["memory", "메모리 검색", "지난 대화를 뜻으로 찾기. 색인하면 대화 글이 임베딩 공급자에게 전송됨", false, false],
				["community", "커뮤니티 플러그인", "직접 만들거나 받은 플러그인을 파일로 설치. 격리된 곳에서 허락한 권한만으로 동작", false, false],
			] as const
		).map(([id, name, description, def, desktopOnly]) => ({ id, name, description, defaultEnabled: def, desktopOnly, enabled: db.feature.feature[id] ?? def })),
	update_feature_setting: ({ id, enabled }) => {
		db.feature.feature[id as string] = enabled as boolean;
		return null;
	},
	get_recoverable_tasks: () => [],
	// ---- 실행 기록 · 사용량 (브라우저 미리보기용 예시) ----
	list_active_tasks: () => [
		{ id: "t-run", taskType: "translate", feature: "translate", runBy: "auto", label: "번역 → ko", owner: "system", ownerRef: null, status: "running", payload: null, result: null, error: null, attemptCount: 1, maxAttempts: 1, createdAt: Date.now() - 2000, startedAt: Date.now() - 2000, updatedAt: Date.now(), finishedAt: null },
	],
	list_task_history: () => {
		const now = Date.now();
		const row = (id: string, feature: string, label: string, status: string, runBy: string, ago: number, u: Partial<Record<string, unknown>> = {}) => ({
			id, taskType: "chat_stream", feature, runBy, label, status, errorCode: null, errorMessage: null, attemptCount: 1,
			createdAt: now - ago, startedAt: now - ago, finishedAt: now - ago + 3200, apiProfileId: "p1", profileName: "Claude", model: "claude-sonnet-5-5",
			calls: 1, inputTokens: 0, outputTokens: 0, characters: 0, images: 0, estimated: false, ...u,
		});
		return [
			row("t1", "chat", "응답 생성", "completed", "user", 60_000, { inputTokens: 5321, outputTokens: 412 }),
			row("t2", "translate", "번역 → ko", "completed", "auto", 58_000, { inputTokens: 480, outputTokens: 390, estimated: true }),
			row("t3", "tts", "음성 읽기", "failed", "auto", 57_000, { profileName: "ElevenLabs", model: "eleven_multilingual_v2", characters: 210, errorCode: "AI_PROVIDER_AUTH_FAILED", errorMessage: "API 키가 맞지 않습니다" }),
			row("t4", "game", "게임 대사", "cancelled", "user", 3_600_000, { calls: 0, profileName: null, model: null }),
			row("t5", "imagegen", "메시지 그림", "completed", "user", 90_000_000, { calls: 2, inputTokens: 900, outputTokens: 120, images: 1 }),
		];
	},
	usage_summary: () => [
		{ apiProfileId: "p1", profileName: "Claude", kind: "chat", calls: 42, failed: 1, inputTokens: 182_340, outputTokens: 20_118, characters: 0, images: 0, estimatedCalls: 6, model: "claude-sonnet-5-5", estimatedCost: 0.85, currency: "$" },
		{ apiProfileId: "p2", profileName: "ElevenLabs", kind: "tts", calls: 12, failed: 1, inputTokens: 0, outputTokens: 0, characters: 3_410, images: 0, estimatedCalls: 12, model: "eleven_v3", estimatedCost: null, currency: null },
		{ apiProfileId: "p3", profileName: "NovelAI", kind: "image", calls: 3, failed: 0, inputTokens: 0, outputTokens: 0, characters: 0, images: 3, estimatedCalls: 3, model: "nai-diffusion-4-5-full", estimatedCost: null, currency: null },
	],
	clear_usage: () => 0,
	allow_budget_once: () => null,
	compact_usage: () => ({ usageRows: 0, tasks: 0 }),
	get_panic_log: () => [{ time: Date.now() - 86_400_000, thread: "tokio-rt-worker", message: "예시: called `Option::unwrap()` on a `None` value", location: "src/service/example.rs:42" }],
	clear_panic_log: () => null,
	check_for_update: () => ({ current: "0.1.0", latest: "0.2.0", newer: true, url: "https://github.com/dily311/sandcastle/releases", notes: "예시", publishedAt: null }),
	check_data: () => ({ ok: true, problems: [], checkedFiles: 42, orphanCount: 2, orphanBytes: 380_000, orphans: [{ path: "characters/avatars/old.png", bytes: 300_000 }, { path: "characters/c9/images/x.webp", bytes: 80_000 }] }),
	list_budget_status: () => [
		{
			limit: { apiProfileId: "gemini", dailyTokens: 100_000, monthlyTokens: 2_000_000, dailyCharacters: null, monthlyCharacters: null, dailyImages: null, monthlyImages: null },
			meters: [
				{ metric: "tokens", period: "day", used: 84_200, limit: 100_000, remaining: 15_800 },
				{ metric: "tokens", period: "month", used: 610_000, limit: 2_000_000, remaining: 1_390_000 },
			],
		},
	],
	upsert_budget_limit: () => null,
	take_budget_warnings: () => [],
	list_model_prices: () => [{ model: "claude-sonnet-5-5", currency: "$", inputPerMtok: 3, outputPerMtok: 15, per1kChars: null, perImage: null }],
	upsert_model_price: () => null,
	delete_model_price: () => null,
	save_png_file: () => null,
	default_app_prompts: () => {
		const d = (text: string) => ({ selected: "", items: [{ id: "", name: "기본값", text }] });
		return { translate: d("You are a translator. Translate into {{lang}}."), imagePrompt: d("Describe ONE scene as an English prompt."), imagePromptNai: d("Danbooru tags."), chatImages: d("[이미지]\n사용 가능한 키: {{keys}}"), autoImage: d("[그림]\n응답 끝에 <image>태그</image>"), gameRules: d("규칙 1\n규칙 2"), sceneRules: d("씬 규칙") };
	},
	cancel_task: () => null,
	...chatHandlers,
	...settingsHandlers,
	...gameHandlers,
	...syncHandlers,
	...communityHandlers,
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

// 지우기는 휴지통 항목을 돌려준다 (2-A-3)
export const handlers = withTrash(base);
