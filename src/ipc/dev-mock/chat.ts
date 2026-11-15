// 브라우저 개발용 가짜 캐릭터·대화 데이터와 커맨드 (실제 백엔드 동작을 흉내)
import type {
	ApiProfiles,
	Character_Serialize,
	ChatTurn_Serialize,
	ConversationFolder,
	Conversation_Serialize,
	Message_Serialize,
	Persona_Serialize,
	PromptPresets_Serialize,
	RegexScript_Serialize,
} from "../bindings";

type Args = Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any
let seq = 100;
const uid = (p: string) => `${p}${seq++}`;
const MIN = 60_000;
const now = () => Date.now();

const character = (id: string, name: string, description: string, extra: Partial<Character_Serialize> = {}): Character_Serialize => ({
	id,
	name,
	avatar: null,
	description,
	alternateDescriptions: [],
	firstMessage: null,
	alternateFirstMessage: [],
	exampleDialogue: null,
	systemPromptOverride: null,
	creatorNotes: null,
	worldInfoIds: [],
	color: null,
	tags: [],
	category: null,
	defaultPersonaId: null,
	defaultPromptPresetId: null,
	favorite: false,
	messageCount: 0,
	createdAt: now() - 90 * 24 * 60 * MIN,
	updatedAt: now(),
	...extra,
});

const characters: Character_Serialize[] = [
	character("sera", "Seraphina", "엘도리아 숲의 수호자.\n숲 깊은 곳의 오두막에서 지친 여행자를 돌봄. 그림자 송곳니 무리로부터 숲을 지키는 오래된 마법의 주인.", {
		tags: ["판타지", "힐러"],
		favorite: true,
		firstMessage: '*당신이 눈을 뜨자, 부드러운 빛 속에서 한 여인이 고개를 숙인다.*\n"눈을 떴군요. 다행이에요… 숲의 짐승들에게 쫓기다 쓰러졌었어요."',
		alternateFirstMessage: ['*오두막 문이 열리고 따뜻한 차 향이 번진다.* "깨어났군요."'],
		creatorNotes: '<h3>세라피나 v2</h3><p>숲 배경 판타지 롤플레이용.<br>첫 메시지 2개.</p>',
	}),
	character("aria", "Aria", "왕국 기사단 견습생. 서툴지만 성실함.", { tags: ["기사"] }),
	character("narr", "나레이터", "모든 이야기의 목소리."),
];

const personas: Persona_Serialize[] = [
	{ id: "default", name: "user", avatar: null, description: "", worldInfoIds: [], tags: [], color: "", isSystemDefault: true, createdAt: 0, updatedAt: 0 },
	{ id: "min", name: "Min", avatar: null, description: "숲에서 길 잃은 여행자", worldInfoIds: [], tags: [], color: "", isSystemDefault: false, createdAt: 0, updatedAt: 0 },
	{ id: "haru", name: "하루", avatar: null, description: "대학생", worldInfoIds: [], tags: [], color: "", isSystemDefault: false, createdAt: 0, updatedAt: 0 },
];

const presets: PromptPresets_Serialize[] = [
	{ id: "default", name: "기본 배치", description: null, presetType: "chat_completion", sourceFormat: "native", configJson: {}, settingsJson: null, compatibilityJson: null, createdAt: 0, updatedAt: 0 },
	{ id: "st", name: "Default (ST)", description: null, presetType: "chat_completion", sourceFormat: "sillytavern", configJson: {}, settingsJson: null, compatibilityJson: null, createdAt: 0, updatedAt: 0 },
];

const apis: ApiProfiles[] = [{ id: "gemini", name: "Gemini", provider: "google", formatType: "google", baseUrl: "", model: "gemini-3.7-flash", createdAt: 0, updatedAt: 0 }];

const folders: ConversationFolder[] = [{ id: "f1", characterId: "sera", name: "본편", sortOrder: 0, createdAt: 0, updatedAt: 0 }];

const regex: RegexScript_Serialize[] = [
	{
		id: "r1",
		name: "별표 지우기",
		findRegex: "/\\*/g",
		replaceString: "",
		trimStrings: [],
		placement: ["ai_output"] as never,
		target: "display" as never,
		scope: "global",
		scopeId: null,
		substituteMacros: "none" as never,
		runOnEdit: false,
		minDepth: null,
		maxDepth: null,
		enabled: false,
		sortOrder: 0,
		sourceFormat: "native",
		extraJson: null,
		createdAt: 0,
		updatedAt: 0,
	},
];

type Conv = Omit<Conversation_Serialize, "messageCount">;
const convs: Conv[] = [];
const messages: Message_Serialize[] = [];

function addConv(c: Partial<Conv> & { title: string; characterId: string }, ago: number): Conv {
	const t = now() - ago * MIN;
	const conv: Conv = {
		id: uid("c"),
		personaId: "min",
		worldInfoIds: [],
		mode: "chat",
		meta: null,
		pinned: false,
		imagesEnabled: true,
		createdAt: t,
		updatedAt: t,
		folderId: null,
		promptPresetId: null,
		apiProfileId: null,
		...c,
	};
	convs.push(conv);
	return conv;
}
function addMsg(conv: Conv, role: "user" | "assistant", content: string, ago: number, meta: Record<string, unknown> = {}) {
	const ch = characters.find((c) => c.id === conv.characterId);
	const t = now() - ago * MIN;
	const m: Message_Serialize = {
		id: uid("m"),
		conversationId: conv.id,
		seq: messages.filter((x) => x.conversationId === conv.id).length + 1,
		role,
		name: role === "assistant" ? (ch?.name ?? null) : (personas.find((p) => p.id === conv.personaId)?.name ?? "user"),
		content,
		characterId: role === "assistant" ? conv.characterId : null,
		meta: (role === "assistant" ? { info: { model: "gemini-3.7-flash" }, ...meta } : meta) as Message_Serialize["meta"],
		createdAt: t,
		updatedAt: t,
	};
	messages.push(m);
	return m;
}

{
	const a = addConv({ title: "숲에서 깨어남", characterId: "sera", pinned: true, folderId: "f1" }, 2);
	addMsg(a, "assistant", '*당신이 눈을 뜨자, 부드러운 빛 속에서 한 여인이 고개를 숙인다.*\n\n"눈을 떴군요. 다행이에요… 숲의 짐승들에게 쫓기다 쓰러졌었어요."', 42, { chapter: "숲에서 깨어남" });
	addMsg(a, "user", "여기가 어디죠? 숲인가요?", 41);
	addMsg(a, "assistant", '"제 오두막이에요. 이 숲, 엘도리아는 제가 지키는 곳이에요. 여기선 아무도 당신을 해치지 못해요."\n*그녀가 따뜻한 차를 건넨다.*', 40);
	addMsg(a, "assistant", "(작가 메모: 다음 장면에서 늑대 무리 등장)", 30, { hidden: true, chapter: "그림자 송곳니" });
	addMsg(a, "assistant", "*멀리서 늑대 울음소리가 들린다. 세라피나의 표정이 굳는다.*", 2, {
		swipeId: 1,
		swipes: [
			{ content: "*창밖으로 바람이 분다.*", meta: {} },
			{ content: "*멀리서 늑대 울음소리가 들린다. 세라피나의 표정이 굳는다.*", meta: {} },
		],
	});
	const b = addConv({ title: "Branch #1", characterId: "sera", folderId: "f1" }, 300);
	addMsg(b, "user", "안녕", 300);
	const f = addConv({ title: "늑대 무리 습격", characterId: "sera", meta: { sillytavern: {} } }, 4000);
	for (let i = 0; i < 6; i++) addMsg(f, i % 2 ? "user" : "assistant", `${i + 1}번째 메시지`, 4000 - i);
	const c = addConv({ title: "성 앞의 약속", characterId: "aria" }, 1500);
	addMsg(c, "assistant", '"기사가 되면 꼭 지켜줄게."', 1500);
	// 폰 대화 (Min의 폰)
	const p = addConv({ title: "Seraphina", characterId: "sera", mode: "phone" }, 8);
	addMsg(p, "assistant", "눈 떴어요?\n어제 숲에서 쓰러져 있길래 데려왔어요", 10);
	addMsg(p, "user", "여기 어디예요??", 9);
	addMsg(p, "assistant", "제 오두막이에요\n조금 더 쉬어요", 8);
}

const view = (c: Conv): Conversation_Serialize => ({ ...c, messageCount: messages.filter((m) => m.conversationId === c.id).length });
const conv = (id: string) => {
	const c = convs.find((c) => c.id === id);
	if (!c) throw { code: "RESOURCE_NOT_FOUND", category: "NOT_FOUND", message: "대화를 찾을 수 없음", retryable: false };
	return c;
};
const msgsOf = (id: string) => messages.filter((m) => m.conversationId === id).sort((a, b) => a.createdAt - b.createdAt);
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

const running = new Map<string, { cancelled: boolean }>();
const REPLIES = [
	'*세라피나가 창밖을 살피며 낮게 속삭인다.* "조용히 해요. 저 소리… 그림자 송곳니예요."',
	'"걱정 말아요." *그녀가 지팡이를 쥔다.* "이 오두막엔 결계가 있어요. 하지만 오래 버티진 못해요."',
	'*희미한 빛이 그녀의 손끝에 모인다.* "같이 가요. 혼자 두지 않을게요."',
];

const PHONE_REPLIES = ["눈 떴어요?\n어제 숲에서 쓰러져 있길래 데려왔어요", "괜찮아요 ㅎㅎ\n천천히 쉬어요", "지금 오두막이에요\n올 때 빵 좀 사다 줄래요?"];

async function generate(convId: string, onEvent: { onmessage: (m: { chunk: string }) => void }) {
	if (running.has(convId)) throw { code: "INVALID_INPUT", category: "VALIDATION", message: "이미 응답을 만드는 중", retryable: false };
	const state = { cancelled: false };
	running.set(convId, state);
	const pool = convs.find((c) => c.id === convId)?.mode === "phone" ? PHONE_REPLIES : REPLIES;
	const text = pool[Math.floor(Math.random() * pool.length)];
	let out = "";
	try {
		await sleep(400);
		for (const piece of text.match(/.{1,3}/gsu) ?? []) {
			if (state.cancelled) break;
			out += piece;
			onEvent.onmessage({ chunk: piece });
			await sleep(35);
		}
	} finally {
		running.delete(convId);
	}
	if (state.cancelled && !out.trim()) throw { code: "AI_REQUEST_CANCELLED", category: "AI_PROVIDER", message: "AI 요청이 취소되었습니다", retryable: false };
	return { content: out, info: { model: "gemini-3.7-flash", apiProfileId: "gemini", genStarted: now(), genFinished: now(), cancelled: state.cancelled } };
}

export const chatHandlers: Record<string, (a: Args) => unknown> = {
	get_all_characters: () => characters,
	get_all_conversations: () => convs.map(view).sort((a, b) => b.updatedAt - a.updatedAt),
	get_conversation: ({ id }) => view(conv(id)),
	get_all_personas: () => personas,
	list_prompt_presets: () => presets,
	get_all_api_profile: () => apis,
	get_conv_settings: () => ({ defaultPersonaId: "min" }),
	list_regex_scripts: () => regex,
	upsert_regex_script: ({ input }) => {
		const i = regex.findIndex((r) => r.id === input.id);
		if (i < 0) {
			const r = { ...input, id: uid("r"), sourceFormat: input.sourceFormat ?? "native", createdAt: now(), updatedAt: now() };
			regex.push(r);
			return r;
		}
		regex[i] = { ...regex[i], ...input, updatedAt: now() };
		return regex[i];
	},
	list_character_images: () => [],
	create_character: ({ input }) => {
		const c = character(uid("ch"), input.name, input.description ?? "");
		characters.push(c);
		return c;
	},
	toggle_favorite_character: ({ id, isFavorite }) => {
		const c = characters.find((c) => c.id === id);
		if (c) c.favorite = isFavorite;
		return null;
	},
	create_conversation: ({ input }) => view(addConv({ ...input, meta: input.meta ?? null }, 0)),
	delete_conversation: ({ id }) => {
		convs.splice(convs.indexOf(conv(id)), 1);
		return null;
	},
	rename_conversation: ({ id, title }) => ((conv(id).title = title), null),
	set_conversation_pinned: ({ id, pinned }) => ((conv(id).pinned = pinned), null),
	move_conversation: ({ id, folderId }) => ((conv(id).folderId = folderId), null),
	set_conversation_options: ({ id, options }) => (Object.assign(conv(id), options), view(conv(id))),
	set_conversation_images_enabled: ({ conversationId, enabled }) => ((conv(conversationId).imagesEnabled = enabled), null),
	duplicate_conversation: ({ id }) => {
		const src = conv(id);
		const { id: _omit, ...rest } = src;
		void _omit;
		const c = addConv({ ...rest, title: `${src.title} (복제)`, pinned: false } as never, 0);
		for (const m of msgsOf(id)) messages.push({ ...m, id: uid("m"), conversationId: c.id });
		return view(c);
	},
	branch_conversation: ({ id, messageId }) => {
		const src = conv(id);
		const list = msgsOf(id);
		const upto = list.findIndex((m) => m.id === messageId);
		const { id: _omit, ...rest } = src;
		void _omit;
		const branchMeta = { ...((src.meta as object) ?? {}), branch: { parentId: id, messageId, seq: list[upto].seq } };
		const c = addConv({ ...rest, title: `${src.title} · #${list[upto].seq} 분기`, pinned: false, meta: branchMeta } as never, 0);
		for (const m of list.slice(0, upto + 1)) messages.push({ ...m, id: uid("m"), conversationId: c.id });
		return view(c);
	},
	export_conversation: ({ id }) => msgsOf(id as string).length,
	patch_conversation_meta: ({ id, patch }) => {
		const c = conv(id);
		const meta: Record<string, unknown> = { ...((c.meta as object) ?? {}) };
		for (const [k, v] of Object.entries(patch as object)) {
			if (v === null) delete meta[k];
			else meta[k] = v;
		}
		c.meta = meta as never;
		c.updatedAt = now();
		return view(c);
	},
	delete_messages: ({ ids }) => {
		let n = 0;
		for (const id of ids as string[]) {
			const i = messages.findIndex((m) => m.id === id);
			if (i >= 0) {
				messages.splice(i, 1);
				n++;
			}
		}
		return n;
	},
	list_conversation_folders: ({ characterId }) => folders.filter((f) => f.characterId === characterId),
	create_conversation_folder: ({ characterId, name }) => {
		const f = { id: uid("f"), characterId, name, sortOrder: folders.length, createdAt: now(), updatedAt: now() };
		folders.push(f);
		return f;
	},
	rename_conversation_folder: ({ id, name }) => {
		const f = folders.find((f) => f.id === id);
		if (f) f.name = name;
		return null;
	},
	delete_conversation_folder: ({ id }) => {
		folders.splice(
			folders.findIndex((f) => f.id === id),
			1,
		);
		for (const c of convs) if (c.folderId === id) c.folderId = null;
		return null;
	},
	get_messages: ({ conversationId }) => msgsOf(conversationId),
	append_message: ({ input }) => {
		const c = conv(input.conversationId);
		const m = addMsg(c, input.role, input.content, 0, input.meta ?? {});
		m.name = input.name ?? m.name;
		return m;
	},
	update_message: ({ id, content, meta }) => {
		const m = messages.find((m) => m.id === id);
		if (m) Object.assign(m, { content, meta, updatedAt: now() });
		return null;
	},
	delete_message: ({ id }) => {
		messages.splice(
			messages.findIndex((m) => m.id === id),
			1,
		);
		return null;
	},
	get_message_prompt: ({ messageId }) => {
		const m = messages.find((m) => m.id === messageId);
		if (!m || !(m.meta as { info?: { genStarted?: number } })?.info?.genStarted) return null;
		return {
			chatOptions: {},
			messages: [
				{ role: "system", content: "[앱 규칙] 캐릭터를 연기한다.", token: 12 },
				{ role: "system", content: characters[0].description, token: 48 },
				{ role: "user", content: "여기가 어디죠?", token: 6 },
			],
		};
	},
	chat_send: async ({ conversationId, text, onEvent }): Promise<ChatTurn_Serialize> => {
		const c = conv(conversationId);
		let user: Message_Serialize | null = null;
		if (text.trim()) user = addMsg(c, "user", text.trim(), 0);
		const r = await generate(conversationId, onEvent);
		const assistant = addMsg(c, "assistant", r.content, 0, { info: r.info });
		c.updatedAt = now();
		return { user, assistant };
	},
	chat_regenerate: async ({ conversationId, swipe, onEvent }): Promise<ChatTurn_Serialize> => {
		const last = msgsOf(conversationId).at(-1);
		if (!last || last.role !== "assistant") throw { code: "INVALID_INPUT", category: "VALIDATION", message: "다시 만들 AI 메시지가 없음", retryable: false };
		const r = await generate(conversationId, onEvent);
		const meta = { ...((last.meta as Record<string, unknown>) ?? {}) } as Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any
		if (swipe) {
			const swipes = meta.swipes?.length ? [...meta.swipes] : [{ content: last.content, meta: {} }];
			swipes.push({ content: r.content, meta: { info: r.info } });
			Object.assign(meta, { swipes, swipeId: swipes.length - 1, info: r.info });
		} else {
			delete meta.swipes;
			delete meta.swipeId;
			meta.info = r.info;
		}
		Object.assign(last, { content: r.content, meta, updatedAt: now() });
		return { user: null, assistant: { ...last } };
	},
	chat_cancel: ({ conversationId }) => {
		const s = running.get(conversationId);
		if (s) s.cancelled = true;
		return !!s;
	},
};

/** 설정 화면 가짜 백엔드가 같은 데이터를 쓰도록 */
export const mockData = { characters, personas, presets, apis, regex, convs, messages, uid, now };
