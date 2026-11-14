// 설정 화면용 가짜 백엔드 (테마·API·프리셋·정규식·데이터·위젯)
import type { DataOverview_Serialize, PromptBlocks_Serialize, PromptPresetDetail_Serialize } from "../bindings";
import { mockData } from "./chat";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Args = Record<string, any>;
const { characters, personas, presets, apis, regex, convs, messages, uid, now } = mockData;

const themes = new Map<string, { name: string; author: string; version: string; cssContent: string }>([
	["catppuccin.json", { name: "Catppuccin", author: "test", version: "1.0.0", cssContent: ":root[data-theme=\"light\"] { --bg: #eff1f5; }" }],
]);
const keys = new Set<string>(["gemini"]);
let autostart = false;
let defaultPersona = "min";
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
			block("default", 4, "chatHistory", "대화 기록"),
			block("default", 5, "jailbreak", "마무리 지시", "답은 3문단 이내.", { position: "in_chat", depth: 1, role: "system" }),
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
	get_theme_list: () => [...themes].map(([filename, t]) => ({ filename, name: t.name, author: t.author, version: t.version })),
	load_theme_content: ({ filename }) => themes.get(filename)?.cssContent ?? "",
	save_theme: ({ filename, content }) => (themes.set(filename, content), null),
	delete_theme: ({ filename }) => (themes.delete(filename), null),

	// ---- 위젯 ----
	is_autostart_enabled: () => autostart,
	set_autostart: ({ enabled }) => ((autostart = enabled), null),

	// ---- 페르소나 기본값 ----
	set_system_default_persona: ({ id }) => ((defaultPersona = id), null),
	get_conv_settings: () => ({ defaultPersonaId: defaultPersona }),

	// ---- API ----
	has_api_key: ({ profileId }) => keys.has(profileId),
	delete_api_key: ({ profileId }) => (keys.delete(profileId), null),
	create_api_profile: ({ input, apiKey }) => {
		const a = { ...input, id: uid("api"), createdAt: now(), updatedAt: now() };
		apis.push(a);
		if (apiKey) keys.add(a.id);
		return a;
	},
	update_api_profile: ({ id, input, apiKey }) => {
		const a = apis.find((x) => x.id === id);
		if (a) Object.assign(a, input, { updatedAt: now() });
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
	get_models: ({ profileId }) => {
		if (!keys.has(profileId)) throw { code: "AI_PROVIDER_AUTH_FAILED", category: "AI_PROVIDER", message: "API 키가 설정되지 않았습니다", retryable: false };
		return ["gemini-3.7-flash", "gemini-3.1-pro-preview", "gemini-2.5-pro"];
	},
	test_message: async ({ profileId }) => {
		await new Promise((r) => setTimeout(r, 600));
		if (!keys.has(profileId)) throw { code: "AI_PROVIDER_AUTH_FAILED", category: "AI_PROVIDER", message: "API 키가 설정되지 않았습니다", retryable: false };
		return "OK";
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
