// 설정 화면용 가짜 백엔드 (테마·API·프리셋·정규식·데이터·위젯)
import type { DataOverview_Serialize, GameProject_Serialize, PhoneContact, PromptBlocks_Serialize, PromptPresetDetail_Serialize } from "../bindings";
import { mockData } from "./chat";
import { gameView } from "./game";

const books = [
	{ id: "wb1", name: "엘다리아 대륙", description: "숲과 왕국 설정", createdAt: 0, updatedAt: Date.now() - 86400000 },
	{ id: "wb2", name: "현대 서울", description: null as string | null, createdAt: 0, updatedAt: Date.now() - 3600000 },
];
const wiEntries = [
	{ id: "we1", bookId: "wb1", comment: "엘도리아 숲", keys: ["엘도리아", "숲"], secondaryKeys: [], content: "고대 마법이 깃든 숲. 세라피나가 지킴.", enabled: true, priority: 100, constant: false },
	{ id: "we2", bookId: "wb1", comment: "그림자 송곳니", keys: ["늑대", "그림자 송곳니"], secondaryKeys: [], content: "숲을 노리는 늑대 무리.", enabled: true, priority: 90, constant: false },
	{ id: "we3", bookId: "wb1", comment: "세계 규칙", keys: [], secondaryKeys: [], content: "이 세계에는 마법이 존재함.", enabled: true, priority: 200, constant: true },
	{ id: "we4", bookId: "wb2", comment: "", keys: ["지하철"], secondaryKeys: [], content: "2호선은 늘 붐빔.", enabled: false, priority: 50, constant: false },
];

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Args = Record<string, any>;
const { characters, personas, presets, apis, regex, convs, messages, uid, now } = mockData;

type MockTheme = { name: string; author: string; version: string; cssContent: string; colors: { light: Record<string, string>; dark: Record<string, string> } };
const themes = new Map<string, MockTheme>([
	["latte.json", { name: "라테", author: "나", version: "1.0.0", cssContent: "", colors: { light: { "--bg": "#f3ede4", "--surface": "#fffaf3", "--text": "#3b2f25" }, dark: { "--bg": "#231c17", "--surface": "#2e251f", "--text": "#f1e6d8" } } }],
]);
const keys = new Set<string>(["gemini"]);
let autostart = false;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let mockMascotSetting: any = {
	characters: [],
	pairChatEnabled: true,
	pairChatIntervalSec: 90,
	pairChatUseAi: false,
	pairChatDistancePx: 260,
};
let defaultPersona = "default";
const contacts: PhoneContact[] = [
	{ ownerKind: "persona", ownerId: "min", contactKind: "character", contactId: "sera", status: "숲의 오두막", sortOrder: 0, createdAt: 0 },
	{ ownerKind: "persona", ownerId: "min", contactKind: "character", contactId: "aria", status: "기사단 훈련 중", sortOrder: 1, createdAt: 0 },
	{ ownerKind: "character", ownerId: "aria", contactKind: "character", contactId: "sera", status: "", sortOrder: 0, createdAt: 0 },
	{ ownerKind: "character", ownerId: "aria", contactKind: "persona", contactId: "min", status: "숲에서 만난 여행자", sortOrder: 1, createdAt: 0 },
];
const gameList: GameProject_Serialize[] = [{ id: "g1", title: "숲의 수수께끼", description: "숲에서 길을 잃은 당신과 견습 마법사의 일주일", templateId: "dating-sim", version: "1.0.0", apiVersion: "1", coverImage: null, startSceneKey: null, startMapId: null, startLocationId: null, worldInfoIds: [], customCss: null, settingsJson: { storyPlan: { enabled: true } }, createdAt: 0, updatedAt: Date.now() - 3_600_000 }];
const session = (id: string, scene: string, ago: number, seq: number) => ({ id, gameId: "g1", playerId: "min", currentSceneId: scene, currentMapId: null, currentZoneId: null, currentLocationId: null, currentWeatherId: null, gameTime: 8 * 3600 + seq * 600, flagsJson: {}, parentSessionId: null, pendingScenesJson: [], inventoryJson: {}, relationshipsJson: {}, stageJson: {}, forkedFromSeq: null, headSeq: seq, createdAt: Date.now() - ago, updatedAt: Date.now() - ago });
const sessions = [session("ss1", "s2", 3_600_000, 12), session("ss2", "s1", 86_400_000 * 2, 3), { ...session("ss3", "s1", 1_800_000, 8), parentSessionId: "ss1", forkedFromSeq: 5 }];
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const packages: Record<string, any> = {};
let pendingRestore: null | { createdAt: number } = null;

const block = (presetId: string, i: number, identifier: string, name: string, content = "", extra: Partial<PromptBlocks_Serialize> = {}): PromptBlocks_Serialize => ({
	id: uid("b"),
	presetId,
	name,
	identifier,
	blockType: "text",
	role: "system",
	content,
	enabled: true,
	position: "relative",
	depth: 0,
	sortOrder: i,
	triggersJson: [],
	extraJson: null,
	createdAt: 0,
	updatedAt: 0,
	...extra,
});
const blocks = new Map<string, PromptBlocks_Serialize[]>([
	[
		"default",
		[
			block("default", 0, "main", "기본 지시", "{{char}}가 되어 {{user}}와 대화를 이어감. 행동은 *별표*, 대사는 \"따옴표\"."),
			block("default", 1, "charDescription", "캐릭터 설명"),
			block("default", 2, "personaDescription", "페르소나 설명"),
			block("default", 3, "worldInfoBefore", "월드인포"),
			block("default", 4, "utils", "유틸 프롬프트", "{{img_inprompt}}\n{{multi_char}}\n{{auto_image}}\n{{vars}}"),
			block("default", 5, "chatHistory", "대화 기록"),
			block("default", 6, "jailbreak", "마무리 지시", "답은 3문단 이내.", { position: "in_chat", depth: 1, role: "system" }),
		],
	],
	["st", [block("st", 0, "main", "Main Prompt", "Write {{char}}'s next reply.")]],
]);

function detail(id: string): PromptPresetDetail_Serialize {
	const preset = presets.find((p) => p.id === id);
	if (!preset) throw { code: "NOT_FOUND", category: "VALIDATION", message: "프리셋 없음", retryable: false };
	return { preset, blocks: [...(blocks.get(id) ?? [])] };
}

export const settingsHandlers: Record<string, (a: Args) => unknown> = {
	// ---- 테마 ----
	get_theme_list: () => [...themes].map(([filename, t]) => ({ filename, name: t.name, author: t.author, version: t.version, colors: t.colors })),
	load_theme: ({ filename }) => {
		const t = themes.get(filename);
		if (!t) throw { code: "IO_ERROR", category: "IO", message: "테마 없음", retryable: false };
		return { filename, ...structuredClone(t) };
	},
	load_theme_content: ({ filename }) => themes.get(filename)?.cssContent ?? "",
	save_theme: ({ filename, content }) => (themes.set(filename, content), null),
	delete_theme: ({ filename }) => (themes.delete(filename), null),

	// ---- 위젯 ----
	is_autostart_enabled: () => autostart,
	set_autostart: ({ enabled }) => ((autostart = enabled), null),
	get_widget_mascot_settings: () => mockMascotSetting,
	save_widget_mascot_settings: ({ setting }) => ((mockMascotSetting = structuredClone(setting)), mockMascotSetting),
	upload_widget_mascot_image: ({ sourcePath }) =>
		sourcePath
			? "data:image/svg+xml," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><circle cx="32" cy="32" r="24" fill="#e9a87c"/><circle cx="24" cy="28" r="3" fill="#2b2118"/><circle cx="40" cy="28" r="3" fill="#2b2118"/><path d="M24 38 Q32 45 40 38" stroke="#2b2118" stroke-width="2.5" fill="none" stroke-linecap="round"/></svg>')
			: "",
	delete_widget_mascot_image: () => null,
	mascot_generate_reply: async ({ request }) => {
		const m = mockMascotSetting.characters.find((c: { id: string }) => c.id === request.mascotId);
		return `${m?.name ?? "시메지"}: "${request.userMessage}"라고 했구나! 헤헤, 반가워!`;
	},
	mascot_generate_peer_chat: async ({ request }) => {
		const a = mockMascotSetting.characters.find((c: { id: string }) => c.id === request.mascotAId);
		const b = mockMascotSetting.characters.find((c: { id: string }) => c.id === request.mascotBId);
		return [
			{ mascotId: request.mascotAId, name: a?.name ?? "A", text: "안녕! 오늘 화면 위가 참 포근하네~" },
			{ mascotId: request.mascotBId, name: b?.name ?? "B", text: "그러게! 같이 조금만 걸어다닐까?" },
			{ mascotId: request.mascotAId, name: a?.name ?? "A", text: "좋아, 출발하자!" },
		];
	},

	// ---- 페르소나 기본값 ----
	set_system_default_persona: ({ id }) => ((defaultPersona = id), null),
	get_conv_settings: () => ({ defaultPersonaId: defaultPersona }),

	// ---- 캐릭터 편집 ----
	update_character: ({ id, input }) => {
		const c = characters.find((x) => x.id === id);
		if (!c) throw { code: "RESOURCE_NOT_FOUND", category: "NOT_FOUND", message: "캐릭터 없음", retryable: false };
		Object.assign(c, input, { updatedAt: now() });
		return c;
	},
	set_character_avatar: ({ id, path }) => {
		const c = characters.find((x) => x.id === id)!;
		c.avatar = path
			? "data:image/svg+xml," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="#cf6a43"/><circle cx="5" cy="4" r="2" fill="#fff"/></svg>')
			: null;
		return c;
	},

	set_character_cover: ({ id, path }) => {
		const c = characters.find((x) => x.id === id)!;
		c.cover = path
			? "data:image/svg+xml," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 10"><defs><linearGradient id="g"><stop offset="0" stop-color="#5b4bdb"/><stop offset="1" stop-color="#f38fb1"/></linearGradient></defs><rect width="40" height="10" fill="url(#g)"/></svg>')
			: null;
		return c;
	},

	set_avatar_fit: ({ owner, id, fit }) => {
		const list: { id: string; avatarFit: unknown }[] = owner === "persona" ? personas : characters;
		const x = list.find((v) => v.id === id);
		if (x) x.avatarFit = fit ?? null;
		return null;
	},
	set_character_cover_position: ({ id, y }) => {
		const c = characters.find((x) => x.id === id)!;
		c.coverY = y as number;
		return null;
	},

	// ---- 복사 ----
	copy_character_to_persona: ({ characterId }) => {
		const c = characters.find((x) => x.id === characterId)!;
		const p = { id: uid("pe"), name: c.name, alias: null, avatar: c.avatar, description: c.description, worldInfoIds: [...c.worldInfoIds], tags: [...c.tags], color: c.color ?? "", isSystemDefault: false, createdAt: now(), updatedAt: now(), avatarFit: c.avatarFit };
		personas.push(p);
		return p;
	},
	copy_persona_to_character: ({ personaId }) => {
		const p = personas.find((x) => x.id === personaId)!;
		const base = characters[0];
		const c = { ...base, id: uid("ch"), name: p.name, avatar: p.avatar, description: p.description, firstMessage: null, alternateFirstMessage: [], creatorNotes: null, tags: [...p.tags], color: p.color || null, favorite: false, createdAt: now(), updatedAt: now() };
		characters.push(c);
		return c;
	},
	game_list: () => gameList,
	list_templates: () => [{ id: "dating-sim", name: "연애 시뮬레이션", apiVersion: "1", templateVersion: "1.0.0", capabilities: { choices: true, freeText: true, inventory: true, map: true, aiNarration: true, location: true, time: true } }],
	create_game: ({ title, templateId }) => {
		const g = { ...gameList[0], id: uid("g"), title, templateId, description: null, createdAt: now(), updatedAt: now() };
		gameList.unshift(g);
		return g;
	},
	delete_game: ({ gameId }) => {
		gameList.splice(
			gameList.findIndex((g) => g.id === gameId),
			1,
		);
		return null;
	},
	close_game: () => null,
	export_game: ({ gameId }) => `C:/Users/me/Downloads/${gameId}.zip`,
	open_game: ({ gameId }) => {
		const project = gameList.find((g) => g.id === gameId);
		if (!project) throw { code: "RESOURCE_NOT_FOUND", category: "NOT_FOUND", message: "게임 없음", retryable: false };
		return {
			project,
			contentHash: "h1",
			assetBaseUrl: `http://game.localhost/${gameId}/`,
			diagnostics: [{ severity: "warning", message: "배경 없는 장소", targets: ["광장"], text: "배경 없는 장소: [광장]" }],
			view: gameView,
		};
	},
	list_sessions: () => sessions,
	delete_sessions: ({ sessionId }) => {
		sessions.splice(
			sessions.findIndex((x) => x.id === sessionId),
			1,
		);
		return null;
	},
	validate_game: () => [],
	get_package_data: ({ gameId }) => {
		const project = gameList.find((g) => g.id === gameId)!;
		packages[gameId] ??= {
			project: { ...project },
			stateDefs: [{ id: "sd1", description: "세라와의 친밀도", key: "affection", type: "number", defaultValue: "0", configJson: "{}", createdAt: 0, updatedAt: 0 }],
			maps: [],
			zones: [],
			locations: [],
			locationConnections: [],
			interactables: [],
			timePeriods: [
				{ id: "t1", gameId, name: "아침", sortOrder: 0, startMinute: 360, endMinute: 720 },
				{ id: "t2", gameId, name: "밤", sortOrder: 1, startMinute: 1080, endMinute: 1440 },
			],
			weatherTypes: [],
			weatherTransitions: [],
			events: [],
			eventConditions: [],
			eventCharacterPool: [],
			encounterTables: [],
			encounterEntries: [],
			scenes: [
				{ id: "s1", sceneName: "숲 입구", description: "길을 잃고 숲에 들어섬", sceneKey: "intro", isEnding: false, endingId: null, settingsJson: null, script: "나레이션: 바람이 분다.\n엘리안: 괜찮아요?", choices: [{ id: "c1", sceneId: "s1", text: "따라간다", nextSceneKey: "cabin", sortOrder: 0 }], branches: [] },
				{ id: "s2", sceneName: "오두막", description: null, sceneKey: "cabin", isEnding: true, endingId: null, settingsJson: null, script: "", choices: [], branches: [] },
			],
			endings: [],
			characters: [
				{ id: "gc1", gameId, characterId: null, name: "엘리안", avatar: null, description: "숲을 지키는 견습 마법사", firstMessage: "괜찮아요?", exampleDialogue: null, systemPromptOverride: null, behaviorType: "companion", worldInfoIds: [], tags: null, settingsJson: {} },
			],
			spawnLocations: [],
			items: [],
			relationshipDefs: [],
			assets: [],
			audioAssets: [],
			characterAssets: [],
			cgAssets: [],
			backgroundAssets: [],
			worldInfoBooks: [],
		};
		return JSON.parse(JSON.stringify(packages[gameId]));
	},
	save_package: ({ gameId, data }) => {
		packages[gameId] = JSON.parse(JSON.stringify(data));
		const g = gameList.find((x) => x.id === gameId);
		if (g) Object.assign(g, { title: data.project.title, description: data.project.description, updatedAt: now() });
		return { deleted: {}, diagnostics: data.scenes.length ? [] : [{ severity: "error", message: "씬이 없음", targets: [], text: "씬이 없음" }] };
	},
	copy_game_character_to_character: ({ gameCharacterId }) => {
		const name = gameCharacterId === "gc1" ? "엘리안" : "루카";
		const c = { ...characters[0], id: uid("ch"), name, avatar: null, description: "게임에서 가져옴", firstMessage: null, alternateFirstMessage: [], creatorNotes: null, tags: [], favorite: false, createdAt: now(), updatedAt: now() };
		characters.push(c);
		return c;
	},
	copy_character_to_game: ({ characterId, gameId }) => ({ id: uid("gc"), gameId, characterId, name: characters.find((c) => c.id === characterId)?.name ?? "", avatar: null, description: null, firstMessage: null, exampleDialogue: null, systemPromptOverride: null, behaviorType: null, worldInfoIds: [], tags: [], settingsJson: {}, createdAt: now(), updatedAt: now() }),

	// ---- 폰 연락처 (주인·연락처가 각각 페르소나나 캐릭터) ----
	list_phone_contacts: ({ ownerKind, ownerId }) => contacts.filter((c) => c.ownerKind === ownerKind && c.ownerId === ownerId),
	add_phone_contact: ({ ownerKind, ownerId, contactKind, contactId }) => {
		let c = contacts.find((x) => x.ownerKind === ownerKind && x.ownerId === ownerId && x.contactKind === contactKind && x.contactId === contactId);
		if (!c) contacts.push((c = { ownerKind, ownerId, contactKind, contactId, status: "", sortOrder: contacts.length, createdAt: now() }));
		return c;
	},
	remove_phone_contact: ({ ownerKind, ownerId, contactKind, contactId }) => {
		const i = contacts.findIndex((x) => x.ownerKind === ownerKind && x.ownerId === ownerId && x.contactKind === contactKind && x.contactId === contactId);
		if (i >= 0) contacts.splice(i, 1);
		return null;
	},
	set_phone_contact_status: ({ ownerKind, ownerId, contactKind, contactId, status }) => {
		const c = contacts.find((x) => x.ownerKind === ownerKind && x.ownerId === ownerId && x.contactKind === contactKind && x.contactId === contactId);
		if (c) c.status = status;
		return null;
	},
	phone_peek: async ({ conversationId, onEvent }) => {
		const conv = convs.find((c) => c.id === conversationId)!;
		const phoneMeta = (conv.meta as { phone?: { owner: { kind: string; id: string }; contact: { kind: string; id: string } } } | null)?.phone;
		const nameOf = (w: { kind: string; id: string }) => (w.kind === "persona" ? personas.find((p) => p.id === w.id)?.name : characters.find((c) => c.id === w.id)?.name) ?? "?";
		const me = phoneMeta ? nameOf(phoneMeta.owner) : "나";
		const other = phoneMeta ? nameOf(phoneMeta.contact) : "상대";
		const text = `${me}: 지금 뭐 해?\n${other}: 훈련 끝나고 쉬는 중\n${other}: 너는?\n${me}: 오두막에서 차 마셔`;
		for (const piece of text.match(/.{1,4}/gsu) ?? []) {
			onEvent.onmessage({ chunk: piece });
			await new Promise((r) => setTimeout(r, 30));
		}
		const out = [];
		for (const [mine, body] of [[true, "지금 뭐 해?"], [false, "훈련 끝나고 쉬는 중\n너는?"], [true, "오두막에서 차 마셔"]] as [boolean, string][]) {
			const t = now();
			const m = { id: uid("m"), conversationId, seq: messages.filter((x) => x.conversationId === conversationId).length + 1, role: mine ? "user" : "assistant", name: mine ? me : other, content: body, characterId: null, meta: { info: { peek: true } }, createdAt: t, updatedAt: t };
			messages.push(m as never);
			out.push(m);
		}
		conv.updatedAt = now();
		return out;
	},
	phone_generate_profile: async () =>
		JSON.stringify({
			handle: "daily_log",
			status: "오늘도 조용히 차 한 잔",
			bio: "소소한 일상과 기록을 남기는 공간\n가끔 산책 사진 올림",
			link: "숲의 오두막",
			followers: "142",
			following: "38",
			posts: [
				{
					caption: "창밖으로 바람이 꽤 선선하게 분다. 오랜만에 따뜻한 차를 우렸다.",
					location: "오두막 창가",
					date: "2시간 전",
					likes: 24,
					comments: [{ author: "이웃", text: "분위기 너무 좋네요!" }],
				},
				{
					caption: "어제 걸었던 길. 다음엔 카메라도 챙겨 나가야지.",
					location: "산책로",
					date: "어제",
					likes: 41,
					comments: [],
				},
			],
		}),

	// ---- 페르소나 ----
	// ---- 로어북 ----
	get_all_worldinfo_books: () => books.map((b) => ({ ...b })),
	worldinfo_entry_counts: () =>
		books.map((b) => {
			const list = wiEntries.filter((e) => e.bookId === b.id);
			return { bookId: b.id, entries: list.length, enabled: list.filter((e) => e.enabled).length };
		}),
	create_worldinfo_book: ({ input }) => {
		const b = { id: `wb${Date.now().toString(36)}`, name: input.name, description: input.description ?? null, createdAt: Date.now(), updatedAt: Date.now() };
		books.push(b);
		return b;
	},
	update_worldinfo_book: ({ id, input }) => (Object.assign(books.find((b) => b.id === id)!, input, { updatedAt: Date.now() }), null),
	delete_worldinfo_book: ({ id }) => {
		books.splice(books.findIndex((b) => b.id === id), 1);
		return null;
	},
	get_worldinfo_entries: ({ bookId }) => wiEntries.filter((e) => e.bookId === bookId).map((e) => ({ category: "", ...e })),
	upsert_worldinfo_entry: ({ input }) => {
		if (!input.keys.length && !input.constant) throw { code: "INVALID_INPUT", category: "VALIDATION", message: "키워드 필요", retryable: false };
		const e = { ...input, id: input.id ?? `we${Date.now().toString(36)}`, comment: input.comment ?? "", category: input.category ?? "" };
		const i = wiEntries.findIndex((x) => x.id === e.id);
		if (i >= 0) wiEntries[i] = e;
		else wiEntries.push(e);
		return { ...e };
	},
	delete_worldinfo_entry: ({ id }) => {
		wiEntries.splice(wiEntries.findIndex((e) => e.id === id), 1);
		return null;
	},
	export_worldinfo_book: ({ id }) => wiEntries.filter((e) => e.bookId === id).length,
	create_persona: ({ input }) => {
		const p = { ...input, avatar: input.avatar ?? null, id: uid("pe"), createdAt: now(), updatedAt: now() };
		personas.push(p);
		return p;
	},
	update_persona: ({ id, input }) => {
		const p = personas.find((x) => x.id === id);
		if (p) Object.assign(p, input, { updatedAt: now() });
		return null;
	},
	delete_persona: ({ id }) => {
		if (id === defaultPersona) throw { code: "VALIDATION_ERROR", category: "VALIDATION", message: "기본 페르소나는 지울 수 없습니다", retryable: false };
		personas.splice(
			personas.findIndex((p) => p.id === id),
			1,
		);
		for (const c of convs) if (c.personaId === id) c.personaId = null as never;
		for (const c of characters) if (c.defaultPersonaId === id) c.defaultPersonaId = null;
		return null;
	},
	get_persona_usage: ({ id }) => ({ characterCount: characters.filter((c) => c.defaultPersonaId === id).length, isSystemDefault: id === defaultPersona }),
	set_persona_avatar: ({ id, path }) => {
		const p = personas.find((x) => x.id === id)!;
		// 브라우저에는 파일이 없으니 고른 경로 대신 그림 하나를 씀
		p.avatar = path
			? "data:image/svg+xml," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="#7262cf"/><circle cx="5" cy="4" r="2" fill="#fff"/><rect x="2" y="7" width="6" height="3" rx="1.5" fill="#fff"/></svg>')
			: null;
		return p;
	},

	// ---- API ----
	has_api_key: ({ profileId }) => keys.has(profileId),
	get_vertex_key_info: () => null,
	delete_api_key: ({ profileId }) => (keys.delete(profileId), null),
	create_api_profile: ({ input, apiKey }) => {
		const a = { ...input, id: uid("api"), customModels: [], verified: false, createdAt: now(), updatedAt: now() };
		apis.push(a);
		if (apiKey) keys.add(a.id);
		return a;
	},
	update_api_profile: ({ id, input, apiKey }) => {
		const a = apis.find((x) => x.id === id);
		if (a) {
			const reset = !!apiKey || a.baseUrl !== input.baseUrl || a.formatType !== input.formatType;
			Object.assign(a, input, { updatedAt: now(), ...(reset ? { verified: false } : {}) });
		}
		if (apiKey) keys.add(id);
		return null;
	},
	delete_api_profile: ({ id }) => {
		apis.splice(
			apis.findIndex((x) => x.id === id),
			1,
		);
		keys.delete(id);
		return null;
	},
	get_ai_capabilities: ({ formatType, model }) => {
		const yes = (reason: string) => ({ status: "supported", reason });
		if (formatType === "claude") return { embeddings: { status: "unsupported", reason: "Claude에는 임베딩 API가 없음 — Voyage AI나 Gemini 연결을 쓰세요" }, tools: yes("도구 호출 지원"), structured: yes("도구 하나를 강제로 불러 JSON을 받음") };
		if (formatType === "openai" && model !== "gpt-4o") return { embeddings: { status: "unknown", reason: "호환 서버라 확인할 수 없음" }, tools: { status: "unknown", reason: "호환 서버라 확인할 수 없음" }, structured: { status: "unknown", reason: "지원하지 않으면 'JSON만 답하라' 지시와 검사로 대신함" } };
		return { embeddings: yes("임베딩 지원"), tools: yes("도구 호출 지원"), structured: yes("스키마 응답 지원") };
	},
	get_models: ({ profileId }) => {
		if (!keys.has(profileId)) throw { code: "AI_PROVIDER_AUTH_FAILED", category: "AI_PROVIDER", message: "API 키가 설정되지 않았습니다", retryable: false };
		return ["gemini-3.7-flash", "gemini-3.1-pro-preview", "gemini-2.5-pro"];
	},
	test_message: async ({ profileId, model }) => {
		await new Promise((r) => setTimeout(r, 600));
		if (!keys.has(profileId)) throw { code: "AI_PROVIDER_AUTH_FAILED", category: "AI_PROVIDER", message: "API 키가 설정되지 않았습니다", retryable: false };
		const a = apis.find((x) => x.id === profileId);
		if (a) a.verified = true;
		return model ? `Hi! (${model})` : "OK";
	},

	// ---- 프리셋 ----
	get_prompt_preset: ({ id }) => detail(id),
	save_prompt_preset: ({ preset, blocks: input }) => {
		let p = presets.find((x) => x.id === preset.id);
		if (!p) {
			p = { ...preset, id: uid("p"), compatibilityJson: null, createdAt: now(), updatedAt: now() };
			presets.push(p!);
		} else Object.assign(p, preset, { updatedAt: now() });
		const id = p!.id;
		blocks.set(
			id,
			(input as PromptBlocks_Serialize[]).map((b, i) => ({ ...b, id: b.id ?? uid("b"), presetId: id, sortOrder: i, createdAt: now(), updatedAt: now() })),
		);
		presets.sort((a, b) => a.name.localeCompare(b.name));
		return detail(id);
	},
	delete_prompt_preset: ({ id }) => {
		presets.splice(
			presets.findIndex((p) => p.id === id),
			1,
		);
		blocks.delete(id);
		return null;
	},

	// ---- 정규식 ----
	delete_regex_script: ({ id }) => {
		regex.splice(
			regex.findIndex((r) => r.id === id),
			1,
		);
		return null;
	},
	preview_regex_script: ({ input, text, charName, userName }) => {
		const m = /^\/(.*)\/([a-z]*)$/s.exec(input.findRegex);
		const [pattern, flags] = m ? [m[1], m[2]] : [input.findRegex, ""];
		const re = new RegExp(pattern, flags); // 잘못된 패턴이면 그대로 throw → 화면에 오류 표시
		const out = text.replace(re, (input.replaceString ?? "").replace(/\{\{match\}\}/g, "$&"));
		return out.replace(/\{\{char\}\}/g, charName ?? "").replace(/\{\{user\}\}/g, userName ?? "");
	},

	// ---- 가져오기 ----
	export_sillytavern_data: () => ({ characters: 3, chats: 5, books: 2, presets: 2, personas: 2, regex: 1, warnings: [] }),
	export_preset_sillytavern: () => null,
	import_sillytavern_data: () => ({ characters: 2, chats: 3, books: 1, presets: 1, regex: 2, personas: 1, warnings: ["TextGen Settings: 4개 건너뜀 (Chat Completion 프리셋만 가져옴)", "단체 채팅 1개 건너뜀 (지원하지 않음)"] }),
	import_sillytavern: ({ path }) => {
		const p = { id: uid("p"), name: String(path).split(/[\\/]/).pop()!.replace(/\.json$/, ""), description: null, presetType: "chat_completion", sourceFormat: "sillytavern", configJson: {}, settingsJson: { temperature: 1 }, compatibilityJson: null, createdAt: now(), updatedAt: now() };
		presets.push(p);
		blocks.set(p.id, [block(p.id, 0, "main", "Main Prompt", "Write the next reply.")]);
		return { kind: "preset", presetId: p.id, name: p.name, blockCount: 1, regexCount: 0, warnings: [] };
	},

	// ---- 데이터 ----
	get_data_overview: (): DataOverview_Serialize => {
		const summary = (c: (typeof convs)[number]) => {
			const ms = messages.filter((m) => m.conversationId === c.id);
			return {
				id: c.id,
				title: c.title,
				mode: c.mode,
				characterId: c.characterId,
				messageCount: ms.length,
				hiddenCount: ms.filter((m) => (m.meta as { hidden?: boolean } | null)?.hidden).length,
				createdAt: c.createdAt,
				updatedAt: c.updatedAt,
				lastMessageAt: ms.length ? Math.max(...ms.map((m) => m.createdAt)) : null,
			};
		};
		const chars = characters.map((ch) => {
			const cs = convs.filter((c) => c.characterId === ch.id).map(summary);
			return {
				id: ch.id,
				name: ch.name,
				avatar: ch.avatar,
				conversationCount: cs.length,
				messageCount: cs.reduce((n, c) => n + c.messageCount, 0),
				imageCount: 0,
				worldInfoBookCount: 0,
				lastActivity: cs.length ? Math.max(...cs.map((c) => c.lastMessageAt ?? c.updatedAt ?? 0)) : null,
				conversations: cs.sort((a, b) => (b.lastMessageAt ?? 0) - (a.lastMessageAt ?? 0)),
			};
		});
		return {
			characters: chars.sort((a, b) => (b.lastActivity ?? 0) - (a.lastActivity ?? 0)),
			orphanConversations: convs.filter((c) => !characters.some((ch) => ch.id === c.characterId)).map(summary),
			totals: {
				characters: characters.length,
				personas: personas.length,
				conversations: convs.length,
				messages: messages.length,
				worldInfoBooks: 0,
				promptPresets: presets.length,
				regexScripts: regex.length,
				characterImages: 0,
				games: 1,
			},
			storage: { databaseBytes: 3_400_000, charactersBytes: 12_800_000, gamesBytes: 5_100_000, themesBytes: 24_000, totalBytes: 21_324_000 },
		};
	},
	delete_conversations: ({ ids }) => {
		let n = 0;
		for (const id of ids as string[]) {
			const i = convs.findIndex((c) => c.id === id);
			if (i < 0) continue;
			convs.splice(i, 1);
			n++;
			for (let k = messages.length - 1; k >= 0; k--) if (messages[k].conversationId === id) messages.splice(k, 1);
		}
		return n;
	},
	reassign_conversations_character: ({ ids, characterId }) => {
		let n = 0;
		for (const id of ids as string[]) {
			const c = convs.find((x) => x.id === id);
			if (!c) continue;
			c.characterId = characterId as string;
			c.folderId = null;
			c.updatedAt = now();
			n++;
		}
		return n;
	},
	export_backup: async ({ path }) => {
		await new Promise((r) => setTimeout(r, 500));
		return { path, bytes: 21_324_000, fileCount: 42, totals: (settingsHandlers.get_data_overview({}) as DataOverview_Serialize).totals };
	},
	prepare_restore: () => {
		pendingRestore = { createdAt: now() - 86_400_000 };
		const totals = (settingsHandlers.get_data_overview({}) as DataOverview_Serialize).totals;
		return { createdAt: pendingRestore.createdAt, appVersion: "0.1.0", totals, willMigrate: false };
	},
	cancel_restore: () => ((pendingRestore = null), null),
	get_pending_restore: () =>
		pendingRestore && {
			format: "sandcastle-backup",
			version: 1,
			appVersion: "0.1.0",
			createdAt: pendingRestore.createdAt,
			schemaVersion: 120,
			migrations: [],
			appDataDir: "",
			totals: (settingsHandlers.get_data_overview({}) as DataOverview_Serialize).totals,
		},
	restart_app: () => {
		console.info("[dev-mock] 앱 다시 시작 (브라우저에서는 무시)");
		return null;
	},
};
