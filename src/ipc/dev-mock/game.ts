// 게임 플레이 화면 확인용 아주 작은 런타임 (브라우저 개발 모드 전용).
// 실제 엔진처럼 계산하지 않고, 정해 둔 씬 두 개와 장소 세 곳으로 흐름만 흉내 낸다.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Args = Record<string, any>;

const line = (kind: "narration" | "dialogue", speaker: string | null, text: string) => ({ kind, speaker, text, cues: [] });

export const gameView = {
	maps: [{ id: "m1", name: "엘도리아 숲", description: "" }],
	zones: [{ id: "z1", mapId: "m1", name: "숲 가장자리" }],
	locations: [
		{ id: "l1", zoneId: "z1", mapId: "m1", name: "숲 입구", description: "오래된 나무 사이로 좁은 길이 이어짐." },
		{ id: "l2", zoneId: "z1", mapId: "m1", name: "오두막", description: "따뜻한 불빛이 새어 나오는 작은 오두막." },
		{ id: "l3", zoneId: "z1", mapId: "m1", name: "샘터", description: "맑은 물이 솟는 샘." },
	],
	connections: [
		{ fromId: "l1", toId: "l2", travelSeconds: 600, settings: null },
		{ fromId: "l1", toId: "l3", travelSeconds: 900, settings: null },
		{ fromId: "l2", toId: "l1", travelSeconds: 600, settings: null },
		{ fromId: "l3", toId: "l1", travelSeconds: 900, settings: null },
	],
	interactables: [{ id: "i1", locationId: "l1", name: "이끼 낀 표지판" }],
	characters: [
		{ id: "gc1", name: "엘리안", avatar: null, description: "숲을 지키는 견습 마법사", firstMessage: null, tags: null, settings: null },
		{ id: "gc2", name: "루카", avatar: null, description: "떠돌이 상인", firstMessage: null, tags: null, settings: null },
	],
	timePeriods: [{ id: "t1", name: "아침", startMinute: 360, endMinute: 720 }],
	weatherTypes: [{ id: "w1", name: "맑음" }],
	scenes: [
		{ id: "s1", sceneKey: "intro", name: "숲 입구", description: "길을 잃고 숲에 들어섬", isEnding: false, settings: null, aiChat: true, choices: [{ id: "c1", text: "목소리를 따라간다", sortOrder: 0 }, { id: "c2", text: "그 자리에서 기다린다", sortOrder: 1 }] },
		{ id: "s2", sceneKey: "cabin", name: "오두막", description: null, isEnding: false, settings: null, aiChat: true, choices: [] },
	],
	items: [{ id: "it1", name: "약초 꾸러미", description: "상처에 바르면 금방 나음", icon: null, kind: "misc", maxStack: 9, isUnique: false, isKey: false, consumable: true, usable: true, settings: null }],
	relationshipAxes: [],
	assets: { characters: [], backgrounds: [], cgs: [], audio: [] },
};

let state = {
	sessionId: "ss1",
	locationId: "l1",
	mapId: "m1",
	zoneId: "z1",
	gameTime: 8 * 3600 + 20 * 60,
	weatherId: "w1",
	timePeriodId: "t1",
	currentSceneId: "s1" as string | null,
	activeNpcId: null as string | null,
	availableChoices: ["c1", "c2"],
	availableMoves: [] as unknown[],
	sceneChat: true,
	flags: {},
	inventory: { it1: 2 } as Record<string, number>,
	relationships: {},
	stage: { background: null, bgm: null, standings: {} },
	presentNpcs: ["gc1"],
};

const intro = [
	{ type: "sceneScript", sceneId: "s1", lines: [line("narration", null, "바람이 나뭇잎을 흔든다. 어디선가 낮은 목소리가 들림."), line("dialogue", "gc1", "괜찮아요? 이 숲에서 혼자 다니면 위험해요."), line("dialogue", "player", "길을 잃은 것 같아요…"), line("dialogue", "gc1", "제 오두막으로 가요. 금방이에요.")] },
];

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const saves: any[] = [{ name: null, seq: 5, sessionId: "ss1", slotNumber: 1, currentSceneId: "s1", currentMapId: null, currentZoneId: null, currentLocationId: "l1", currentWeatherId: null, gameTime: 8 * 3600, stateSnapshot: {}, updatedAt: Date.now() - 3600_000 }];
const sessionOptions: Record<string, Record<string, unknown>> = {};
let replySeq = 0;

const respond = (events: unknown[]) => ({ state: structuredClone(state), events });

export const gameHandlers: Record<string, (a: Args) => unknown> = {
	start_new_game: () => respond(intro),
	continue_game: () => ({ ...respond(intro), history: { entries: [], hasMore: false, oldestSeq: null } }),
	dispatch_command: ({ command }) => {
		state.gameTime += 300;
		// c2(기다린다)는 다음 씬이 없는 선택지 → 씬이 이어지고 프론트가 sceneSay로 서술을 받는다
		if (command.type === "choose" && command.choiceId === "c2") {
			return respond([{ type: "choiceMade", choiceId: "c2", nextSceneId: null, effect: null, continuesScene: true }]);
		}
		if (command.type === "endScene") {
			state.currentSceneId = null;
			state.availableChoices = [];
			state.sceneChat = false;
			return respond([{ type: "sceneEndRequested" }, { type: "stageLines", lines: [line("narration", null, "장면이 끝났다. 이제 자유롭게 움직일 수 있다.")] }]);
		}
		if (command.type === "choose") {
			state.currentSceneId = "s2";
			state.availableChoices = [];
			state.locationId = "l2";
			return respond([
				{ type: "choiceMade", choiceId: "c1", nextSceneId: "s2", effect: null, continuesScene: false },
				{ type: "itemAdded", itemId: "it1", quantity: 1 },
				{ type: "sceneScript", sceneId: "s2", lines: [line("narration", null, "작은 오두막 안, 장작이 타닥거린다."), line("dialogue", "gc1", "여기서 쉬어요. 차를 내올게요.")] },
			]);
		}
		if (command.type === "moveTo") {
			state.locationId = command.locationId;
			state.currentSceneId = null;
			state.gameTime += 600;
			return respond([{ type: "locationChanged", from: "l1", to: command.locationId }]);
		}
		if (command.type === "advanceTime") state.gameTime += command.minutes * 60;
		if (command.type === "talk") state.activeNpcId = command.gameCharacterId;
		if (command.type === "stopTalk") state.activeNpcId = null;
		if (command.type === "interact") return respond([{ type: "stageLines", lines: [line("narration", null, "표지판에는 '오두막 →'이라고 희미하게 적혀 있음.")] }]);
		return respond([]);
	},
	scene_say: ({ input }) => {
		if (!state.currentSceneId || !state.sceneChat) throw { code: "GAME_ACTION_BLOCKED", category: "GAME", message: "열린 씬이 없어 씬 입력을 쓸 수 없습니다", retryable: false, details: { reason: "noOpenScene" } };
		return {
			state: structuredClone(state),
			events: [{ type: "choicePresented", choices: ["오두막으로 가 보자고 한다", "조금 더 쉬었다 간다"] }],
			lines: [line("dialogue", "player", input), line("narration", null, "잠시 정적이 흐르고, 엘리안이 고개를 끄덕인다."), line("dialogue", "gc1", "그럼 조금만 더 기다려 봐요. 해가 뜨면 길이 보일 거예요.")],
			text: `나: ${input}
잠시 정적이 흐르고, 엘리안이 고개를 끄덕인다.
엘리안: 그럼 조금만 더 기다려 봐요.`,
			messageId: `gm${++replySeq}`,
			userMessageId: `gu${replySeq}`,
		};
	},
	npc_say: ({ input }) => ({
		state: structuredClone(state),
		events: [{ type: "choicePresented", choices: ["차 고마워요", "여긴 어디예요?"] }],
		lines: [line("dialogue", "player", input), line("dialogue", state.activeNpcId ?? "gc1", "천천히 마셔요. 몸이 따뜻해질 거예요.")],
		text: "엘리안: 천천히 마셔요. 몸이 따뜻해질 거예요.",
		messageId: `gm${++replySeq}`,
		userMessageId: `gu${replySeq}`,
	}),
	edit_game_message: ({ messageId, content }) => ({
		id: messageId,
		text: content,
		lines: (content as string).split("\n").filter((l) => l.trim()).map((l) => {
			const m = /^([^:]+):\s*(.*)$/.exec(l);
			return m ? line("dialogue", m[1].trim() === "엘리안" ? "gc1" : null, m[2]) : line("narration", null, l);
		}),
	}),
	regenerate_game_reply: () => ({
		state: structuredClone(state),
		events: [],
		lines: [line("dialogue", state.activeNpcId ?? "gc1", "(다시) 차가 식기 전에 드세요.")],
		text: "엘리안: (다시) 차가 식기 전에 드세요.",
		messageId: `gm${++replySeq}`,
		userMessageId: `gu${replySeq}`,
	}),
	read_journal: () => [],
	read_transcript: () => ({
		entries: [{ seq: 1, gameTime: 8 * 3600, kind: "sceneScript", sceneId: "s1", role: null, lines: intro[0].lines }],
		hasMore: false,
		oldestSeq: 1,
	}),
	save_list: () => saves,
	save_write: ({ slot }) => {
		const i = saves.findIndex((v) => v.sessionId === state.sessionId && v.slotNumber === slot);
		const v = { name: null, seq: 12, sessionId: state.sessionId, slotNumber: slot as number, currentSceneId: state.currentSceneId, currentMapId: null, currentZoneId: null, currentLocationId: state.locationId, currentWeatherId: null, gameTime: state.gameTime, stateSnapshot: {}, updatedAt: Date.now() };
		if (i >= 0) saves[i] = v;
		else saves.push(v);
		return null;
	},
	load_save: ({ sessionId }) => ({ ...respond([]), state: { ...structuredClone(state), sessionId }, history: { entries: [], hasMore: false, oldestSeq: null } }),
	get_game_session_options: ({ sessionId }) => ({ apiProfileId: null, promptPresetId: null, playerId: "min", ...(sessionOptions[sessionId as string] ?? {}) }),
	set_game_session_options: ({ sessionId, apiProfileId, promptPresetId, playerId }) => {
		const cur = sessionOptions[sessionId as string] ?? {};
		sessionOptions[sessionId as string] = { ...cur, apiProfileId, promptPresetId, ...(playerId ? { playerId } : {}) };
		return null;
	},
	game_context_usage: () => ({ used: 2100, system: 1400, history: 700, historyCount: 9, limit: 16384, reserved: 1024, presetId: "default", presetName: "기본 배치" }),
	reset: () => {
		state = { ...state, currentSceneId: "s1", availableChoices: ["c1", "c2"], locationId: "l1" };
		return null;
	},
};
