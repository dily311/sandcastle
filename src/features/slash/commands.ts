// 슬래시 커맨드 → 앱 기능 (5-E-20-5-2·5-E-20-6-2, ADR-025). 엔진(script.ts)은 앱을 모르고, 여기서 커맨드 표를 만든다.
// 스크립트는 사용자가 입력칸에서 보내거나 빠른 답장을 눌렀을 때만 돈다.
// AI를 부르는 커맨드는 한 번 실행에 설정의 상한(기본 5, 최대 50)번까지 — 넘으면 오류로 멈춘다.
// 변수는 실행 중에 들고 있다가 끝날 때 한 번 저장한다 (반복 안에서 여러 번 바꿔도 저장은 한 번).
import { call, commands } from "$shared/ipc/client";
import { promptText } from "$shared/stores/confirm.svelte";
import { toasts } from "$shared/stores/toast.svelte";
import { pluginSetting } from "$shared/stores/aiTools.svelte";
import { library } from "$features/character/library.svelte";
import type { ChatSession } from "$features/character/session.svelte";
import { quickReplies } from "./quickReply.svelte";
import { addTo, getIndexed, runScript, ScriptError, setIndexed, type ArgValue, type Handler, type ScriptEnv } from "./script";

const src = { source: "slash" };
/** 한 번 실행 AI 호출 상한 (설정이 없을 때, ADR-025 5-E-20-6-0) */
export const DEFAULT_AI_LIMIT = 5;
export const MAX_AI_LIMIT = 50;
export function aiLimit(): number {
	const v = Number((pluginSetting.value?.quickReply as { aiLimit?: number } | undefined)?.aiLimit ?? DEFAULT_AI_LIMIT);
	return Math.min(MAX_AI_LIMIT, Math.max(1, Number.isFinite(v) ? Math.round(v) : DEFAULT_AI_LIMIT));
}
/** SillyTavern에 있지만 지원하지 않는 커맨드 (ADR-025) */
const UNSUPPORTED = ["import", "fetch", "js", "api", "qr-set", "bg", "theme", "css", "member-add", "member-remove", "member-get", "member-count", "group"];

/** "세트.버튼" → 그 빠른 답장의 스크립트 (세트 이름에 점이 있어도 되게 점마다 나눠 본다) */
export function quickReplySource(name: string): string | undefined {
	const n = name.trim();
	for (let i = n.indexOf("."); i > 0; i = n.indexOf(".", i + 1)) {
		const [setName, label] = [n.slice(0, i).trim(), n.slice(i + 1).trim()];
		const item = quickReplies.sets.find((s) => s.name === setName)?.items.find((q) => q.label === label);
		if (item) return item.message;
	}
	return undefined;
}

const text = (v: ArgValue | undefined) => (typeof v === "string" ? v : "");

/** "5" 또는 "3-7" → 번호(seq) 목록. 비면 마지막 메시지 */
function range(arg: string, last: number): number[] {
	const m = /^(\d+)(?:\s*-\s*(\d+))?$/.exec(arg.trim());
	if (!m) return arg.trim() ? [] : [last];
	const [a, b] = [Number(m[1]), Number(m[2] ?? m[1])];
	return Array.from({ length: Math.max(0, b - a + 1) }, (_, i) => a + i);
}

/** 이 대화에서 스크립트 실행 환경. flush: 바뀐 변수 저장 (실행이 끝나면 부름) */
export function slashEnv(session: ChatSession, opts: { input?: string } = {}): ScriptEnv & { flush: () => Promise<void> } {
	const convId = session.convId;
	const conv = () => library.conversation(convId);
	const charName = () => library.character(conv()?.characterId)?.name ?? "";
	const userName = () => library.persona(conv()?.personaId)?.name ?? "";
	// 변수: 대화 meta.variables (SillyTavern chat_metadata.variables와 같은 범위). 실행 중엔 여기 들고 있다가 바뀔 때 저장
	const vars: Record<string, string> = Object.fromEntries(
		Object.entries((conv()?.meta as { variables?: Record<string, unknown> } | null)?.variables ?? {}).map(([k, v]) => [k, typeof v === "string" ? v : JSON.stringify(v)]),
	);
	const globals: Record<string, string> = { ...((pluginSetting.value?.quickReply as { globals?: Record<string, string> } | undefined)?.globals ?? {}) };
	let varsDirty = false;
	let globalsDirty = false;
	const limit = aiLimit();
	let aiCalls = 0;
	const ai = (name: string) => {
		if (++aiCalls > limit) throw new ScriptError(`AI 호출이 한 번 실행 상한(${limit}번)을 넘어 /${name}에서 멈춤 — 설정 > 빠른 답장에서 바꿀 수 있음`);
	};
	/** 변수 커맨드 묶음 (대화 변수·전역 변수가 같은 모양) */
	const varCommands = (store: Record<string, string>, dirty: () => void, prefix: "" | "global"): Record<string, Handler> => {
		const keyOf = (named: Record<string, ArgValue>, unnamed: ArgValue) => (text(named.key) || text(unnamed)).trim();
		const set = (key: string, value: string, index?: string) => {
			store[key] = index !== undefined ? setIndexed(store[key] ?? "", index, value) : value;
			dirty();
			return store[key];
		};
		const step = (key: string, by: number) => {
			const n = Number(store[key] ?? 0);
			return set(key, String((Number.isFinite(n) ? n : 0) + by));
		};
		return {
			[`set${prefix}var`]: ({ named, unnamed }) => {
				const key = text(named.key).trim();
				if (!key) throw new ScriptError(`/set${prefix}var: key=이름 이 필요함`);
				set(key, text(unnamed), named.index === undefined ? undefined : text(named.index));
			},
			[`get${prefix}var`]: ({ named, unnamed }) => {
				const v = store[keyOf(named, unnamed)] ?? "";
				return named.index === undefined ? v : (getIndexed(v, text(named.index)) ?? "");
			},
			[`add${prefix}var`]: ({ named, unnamed }) => {
				const key = text(named.key).trim();
				if (!key) throw new ScriptError(`/add${prefix}var: key=이름 이 필요함`);
				return set(key, addTo(store[key] ?? "", text(unnamed)));
			},
			[`inc${prefix}var`]: ({ named, unnamed }) => step(keyOf(named, unnamed), 1),
			[`dec${prefix}var`]: ({ named, unnamed }) => step(keyOf(named, unnamed), -1),
			[`flush${prefix}var`]: ({ named, unnamed }) => {
				delete store[keyOf(named, unnamed)];
				dirty();
			},
		};
	};
	const nextSeq = () => (session.messages.at(-1)?.seq ?? 0) + 1;
	const append = async (role: "user" | "assistant" | "system", content: string, name: string | null = null) => {
		if (!content.trim()) throw new ScriptError("보낼 글이 비어 있음");
		const character = role === "assistant" ? (conv()?.characterId ?? null) : null;
		const m = await call(commands.appendMessage({ conversationId: convId, seq: nextSeq(), role, name, content, characterId: character, meta: null }), src);
		session.messages.push(m);
		void library.refresh(convId);
		return content;
	};
	const bySeq = (seqs: number[]) => session.messages.filter((m) => seqs.includes(m.seq));

	const commandsTable: Record<string, Handler> = {
		// 보내기 (AI를 부르지 않음)
		send: ({ unnamed }) => append("user", text(unnamed)),
		sys: ({ unnamed }) => append("system", text(unnamed)),
		narrator: ({ unnamed }) => append("system", text(unnamed)),
		sendas: ({ named, unnamed }) => {
			const name = text(named.name).trim() || charName();
			return append("assistant", text(unnamed), name === charName() ? null : name);
		},
		// AI 응답 (비용)
		trigger: async () => {
			ai("trigger");
			await session.send("");
		},
		continue: async () => {
			ai("continue");
			await session.send("");
		},
		regen: async () => {
			ai("regen");
			await session.regenerate(false);
		},
		swipe: async () => {
			ai("swipe");
			await session.regenerate(true);
		},
		gen: async ({ unnamed }) => {
			ai("gen");
			return call(commands.slashGenerate(convId, text(unnamed)), src);
		},
		// 메시지 정리
		del: async ({ unnamed }) => {
			const n = Math.max(1, Number(text(unnamed)) || 1);
			await session.removeMany(session.messages.slice(-n).map((m) => m.id));
		},
		hide: async ({ unnamed }) => {
			for (const m of bySeq(range(text(unnamed), session.messages.at(-1)?.seq ?? 0))) await session.setHidden(m, true);
		},
		unhide: async ({ unnamed }) => {
			for (const m of bySeq(range(text(unnamed), session.messages.at(-1)?.seq ?? 0))) await session.setHidden(m, false);
		},
		// 알림·입력·변수
		echo: ({ unnamed }) => {
			toasts.info(text(unnamed));
		},
		input: async ({ named, unnamed }) => {
			const v = await promptText({ title: text(unnamed) || "입력", input: { value: text(named.default), placeholder: text(named.placeholder) } });
			return v ?? "";
		},
		...varCommands(vars, () => (varsDirty = true), ""),
		...varCommands(globals, () => (globalsDirty = true), "global"),
	};

	return {
		commands: commandsTable,
		unsupported: UNSUPPORTED,
		quickReply: quickReplySource,
		variable: (name) => vars[name] ?? globals[name],
		flush: async () => {
			if (varsDirty) await library.patchMeta(convId, { variables: { ...vars } });
			if (globalsDirty && pluginSetting.value) {
				const cur = pluginSetting.value;
				await pluginSetting.save({ ...cur, quickReply: { ...(cur.quickReply ?? {}), globals: { ...globals } } } as typeof cur);
			}
			varsDirty = globalsDirty = false;
		},
		macro: (name, arg) => {
			switch (name) {
				case "char":
					return charName();
				case "user":
					return userName();
				case "getvar":
					return vars[(arg ?? "").trim()] ?? "";
				case "getglobalvar":
					return globals[(arg ?? "").trim()] ?? "";
				case "lastmessage":
					return session.messages.at(-1)?.content ?? "";
				case "newline":
					return "\n";
				case "input":
					return opts.input ?? "";
				default:
					return undefined;
			}
		},
	};
}

/** 입력칸·빠른 답장에서 실행. 오류는 알림으로 (멈춘 것은 조용히) */
export async function runSlash(source: string, session: ChatSession, opts: { input?: string } = {}): Promise<void> {
	if (session.busy) return void toasts.error("응답을 받는 중이라 실행하지 않음");
	if (!quickReplies.loaded) await quickReplies.load().catch(() => {});
	const env = slashEnv(session, opts);
	try {
		await runScript(source, env);
	} catch (e) {
		if (e instanceof ScriptError) toasts.error(e.message);
		// 다른 오류는 call이 이미 알림을 띄움
	} finally {
		// 멈추거나 실패해도 그때까지 바꾼 변수는 남긴다
		await env.flush().catch(() => {});
	}
}

/** 입력이 슬래시 커맨드인가 (`/`로 시작, `//`·`/ ` 같은 글은 아님) */
export function isSlash(text: string): boolean {
	return /^\/[A-Za-z]/.test(text.trimStart());
}
