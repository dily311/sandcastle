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
		selectedApiProfile: "gemini",
		selectedModel: null,
		reasoningEffort: null,
		selectedCharacterId: null,
		selectedPersonaId: "default",
		lastPlayedGameId: null,
		phoneModePrompt: "[폰 메신저 대화]",
		usageRetentionDays: null,
		trashRetentionDays: 30,
		checkUpdatesOnStart: false,
		cgGroupVariants: false,
		doneNotify: { enabled: true, initializedOn: true, title: "", icon: null, soundMode: "default", soundPath: null, soundVolume: 0.8 },
		aiCallLog: false,
		typing: { style: "blink", text: "", image: null },
		worldInfo: { scanDepth: 2, maxRecursionDepth: 0 },
		home: { startOnHome: false, greeting: "", blocks: ["continue", "quick", "favorites", "games"], css: "" },
	} satisfies AppSetting_Serialize as AppSetting_Serialize,
	themeSetting: {
		mode: "system",
		selectTheme: "default",
		isFrameless: true,
		accentColor: "seaglass",
		fontSize: 16,
		grayTone: "slate",
		chat: {
			showTimestamps: false,
			showAvatars: true,
			showMultiCharAvatars: false,
			charAvatarFrame: "circle",
			charAvatarFrameImage: null,
			userAvatarFrame: "circle",
			userAvatarFrameImage: null,
		},
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
		const helper = `<script>(()=>{const post=(a,t)=>parent.postMessage({type:"sc-action",action:a,text:String(t??"").slice(0,a==="slash"?20000:4000)},"*");const slash=t=>(post("slash",t),Promise.resolve(""));window.sandcastle=Object.freeze({fill:t=>post("fill",t),send:t=>post("send",t),choose:t=>post("send",t),slash});window.triggerSlash=slash;addEventListener("click",e=>{const el=e.target&&e.target.closest?e.target.closest("[data-sc-send],[data-sc-choice],[data-sc-fill]"):null;if(!el)return;e.preventDefault();const s=el.getAttribute("data-sc-send")??el.getAttribute("data-sc-choice");if(s!==null)post("send",s||el.textContent.trim());else post("fill",el.getAttribute("data-sc-fill")||el.textContent.trim())},true)})();<\/script>`;
		return URL.createObjectURL(new Blob([helper + doc + report], { type: "text/html" }));
	},
	get_app_setting: () => db.appSetting,
	update_app_setting: ({ patch }) => (db.appSetting = { ...db.appSetting, ...(patch as object) }),
	get_theme_setting: () => db.themeSetting,
	update_theme_setting: ({ patch }) => (db.themeSetting = { ...db.themeSetting, ...(patch as object) }),
	upload_avatar_frame: ({ target, path }) => {
		const chat = { ...(db.themeSetting.chat ?? {}) };
		if (target === "char") {
			chat.charAvatarFrame = "custom";
			chat.charAvatarFrameImage = String(path);
		} else {
			chat.userAvatarFrame = "custom";
			chat.userAvatarFrameImage = String(path);
		}
		return (db.themeSetting = { ...db.themeSetting, chat });
	},
	request_notify_permission: () => true,
	test_notify: () => undefined,
	set_notify_sound: ({ path }) => {
		const cur = db.appSetting.doneNotify;
		const soundPath = (path as string | null) ?? null;
		const soundMode = soundPath ? "custom" : cur.soundMode === "custom" ? "default" : cur.soundMode;
		return (db.appSetting = { ...db.appSetting, doneNotify: { ...cur, soundPath, soundMode } });
	},
	// 미리보기: 복사하지 않고 받은 주소를 그대로 (data:·https: 그림으로 확인)
	set_typing_image: ({ path }) => {
		const cur = db.appSetting as { typing: { style: "blink" | "bounce" | "none" | "image"; text: string } };
		return (db.appSetting = { ...db.appSetting, typing: { ...cur.typing, image: (path as string | null) ?? null } });
	},
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
				["status", "상태창·변수", "대화의 변수를 보고 고치고, 메시지마다 뜨는 상태창을 한 곳에 모아 보여줌", false, false],
				["sync", "동기화", "다른 기기와 파일로 주고받아 합치기. API 키는 넘어가지 않음", false, false],
				["lint", "메시지 linter", "AI 응답이 저장될 때 닫히지 않은 태그·따옴표 등을 검사", false, false],
				["ghostwrite", "대필", "내가 적은 초안을 AI가 내 말투의 다음 대사로 써서 입력칸에 채움", false, false],
				["slash", "슬래시 커맨드·빠른 답장", "입력칸에서 /로 시작하는 커맨드와 스크립트 실행 (SillyTavern 방식). /gen 같은 AI 커맨드는 실행할 때마다 응답 하나의 요금이 듦", false, false],
				["rewrite", "자동 재생성", "linter가 금지어·단어 반복 오류를 찾으면 응답을 새 스와이프로 다시 만듦", false, false],
				["summary", "메모리 확장 · 대화 요약", "프롬프트 창 밖으로 밀려난 오래된 대화를 줄거리로 요약해 AI에게 전달", false, false],
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
	// 보낸 글·받은 글 (5-E-20-6-7): t1(응답 생성)에 잘린 호출 하나
	list_ai_call_log_tasks: () => ["t1"],
	get_ai_call_logs: ({ taskId }) =>
		taskId === "t1"
			? [
					{
						id: 1, taskId: "t1", profileId: "p1", model: "claude-sonnet-5-5", createdAt: Date.now() - 60_000, error: null, finishReason: "max_tokens",
						requestJson: JSON.stringify({ messages: [{ role: "system", content: "너는 세라피나다. 엘도리아 숲의 수호자." }, { role: "user", content: "여기가 어디죠?" }], options: { max_tokens: 300 } }),
						responseText: "\"제 오두막이에요. 이 숲, 엘도리아는 제가 지키는 곳이에요.\" *그녀가 따뜻한 차를 건넨",
					},
				]
			: [],
	clear_ai_call_logs: () => 1,
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
			limit: { apiProfileId: "gemini", monthlyUsd: 20, legacyMonthlyTokens: null },
			meter: { usedUsd: 6.4, limitUsd: 20, remainingUsd: 13.6, unpricedCalls: 2 },
		},
	],
	upsert_budget_limit: () => null,
	add_api_profile_model: ({ model }: { model?: unknown }) => [String(model ?? "").trim()].filter(Boolean),
	remove_api_profile_model: () => [],
	take_budget_warnings: () => [],
	get_model_price: ({ model }: { model?: unknown }) => (String(model).startsWith("claude") ? { input: 2, output: 10, builtin: true } : null),
	get_provider_quota: ({ profileId }: { profileId?: unknown }) =>
		profileId === "elevenlabs"
			? [{ label: "남은 크레딧", value: "8,420" }, { label: "사용", value: "1,580 / 10,000" }, { label: "초기화", value: "2026-12-01" }]
			: [{ label: "구독", value: "Opus" }, { label: "Anlas", value: "10,000" }, { label: "무제한 그림", value: "가능" }],
	list_model_prices: () => [{ model: "claude-sonnet-5-5", currency: "$", inputPerMtok: 3, outputPerMtok: 15, per1kChars: null, perImage: null }],
	upsert_model_price: () => null,
	delete_model_price: () => null,
	save_png_file: () => null,
	default_app_prompts: () => {
		const d = (text: string) => ({ selected: "", items: [{ id: "", name: "기본값", text }] });
		return {
			translate: d("You are a translator. Translate into {{lang}}."),
			imagePrompt: d("Describe ONE scene as an English prompt."),
			imagePromptNai: d("Danbooru tags."),
			chatImages: d("[이미지]\n사용 가능한 키: {{img_keys}}"),
			autoImage: d("[그림]\n응답 끝에 <image>태그</image>"),
			gameRules: d("규칙 1\n규칙 2"),
			sceneRules: d("씬 규칙"),
			summary: d("대화를 요약하는 지시. {max}토큰 안에"),
			multiChar: d("[여러 캐릭터]\n등장인물: {{names}}"),
			vars: d("[변수]\n{{var_list}}"),
			ghost: d("[Ghostwrite request]\nWrite what {{user}} says next in {{user}}'s voice (do not write {{char}}). Length: {{length}}."),
			phoneProfile: d("지금까지의 대화와 캐릭터 설정을 바탕으로 '{{char}}'의 카카오톡 스타일 메신저 프로필 꾸미기 정보를 JSON으로만 생성해줘."),
			phoneSns: d("지금까지의 대화와 캐릭터 설정을 바탕으로 '{{char}}'의 인스타그램 스타일 개인 SNS 페이지를 JSON으로만 생성해줘."),
		};
	},
	cancel_task: () => null,
	...chatHandlers,
	...settingsHandlers,
	...gameHandlers,
	...syncHandlers,
	...communityHandlers,
	// ---- 위젯 (브라우저에서 위젯 화면 확인용) ----
	get_widget_music_settings: () => ({ resize: false, isMiniPlayer: false, isImgEdit: false, isPlayListOpen: false, pos: null, selectedFolder: null, currentPlaylistId: "default", currentSongIndex: 0, audio: { muted: false, volume: 0.7, loopMode: 0, isShuffle: false } }),
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
