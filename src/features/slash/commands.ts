// 슬래시 커맨드 → 앱 기능 (5-E-20-5-2, ADR-025). 엔진(script.ts)은 앱을 모르고, 여기서 커맨드 표를 만든다.
// 스크립트는 사용자가 입력칸에서 보내거나 빠른 답장을 눌렀을 때만 돈다. AI를 부르는 커맨드는 한 번 실행에 MAX_AI_CALLS번까지.
import { call, commands } from "$shared/ipc/client";
import { promptText } from "$shared/stores/confirm.svelte";
import { toasts } from "$shared/stores/toast.svelte";
import { library } from "$features/character/library.svelte";
import type { ChatSession } from "$features/character/session.svelte";
import { runScript, ScriptError, type ArgValue, type Handler, type ScriptEnv } from "./script";

const src = { source: "slash" };
/** 한 번 실행에서 AI를 부를 수 있는 횟수 (반복이 없어 글에 보이는 만큼이지만, 실수로 늘어나지 않게) */
export const MAX_AI_CALLS = 5;
/** SillyTavern에 있지만 지원하지 않는 커맨드 (ADR-025) */
const UNSUPPORTED = ["while", "times", "for", "run", "import", "fetch", "js", "setglobalvar", "getglobalvar", "api", "model", "preset", "qr"];

const text = (v: ArgValue | undefined) => (typeof v === "string" ? v : "");

/** "5" 또는 "3-7" → 번호(seq) 목록. 비면 마지막 메시지 */
function range(arg: string, last: number): number[] {
	const m = /^(\d+)(?:\s*-\s*(\d+))?$/.exec(arg.trim());
	if (!m) return arg.trim() ? [] : [last];
	const [a, b] = [Number(m[1]), Number(m[2] ?? m[1])];
	return Array.from({ length: Math.max(0, b - a + 1) }, (_, i) => a + i);
}

/** 이 대화에서 스크립트 실행 환경 */
export function slashEnv(session: ChatSession): ScriptEnv {
	const convId = session.convId;
	const conv = () => library.conversation(convId);
	const charName = () => library.character(conv()?.characterId)?.name ?? "";
	const userName = () => library.persona(conv()?.personaId)?.name ?? "";
	// 변수: 대화 meta.variables (SillyTavern chat_metadata.variables와 같은 범위). 실행 중엔 여기 들고 있다가 바뀔 때 저장
	const vars: Record<string, string> = { ...(((conv()?.meta as { variables?: Record<string, unknown> } | null)?.variables ?? {}) as Record<string, string>) };
	let aiCalls = 0;
	const ai = () => {
		if (++aiCalls > MAX_AI_CALLS) throw new ScriptError(`AI 호출은 한 번 실행에 ${MAX_AI_CALLS}번까지`);
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
			ai();
			await session.send("");
		},
		continue: async () => {
			ai();
			await session.send("");
		},
		regen: async () => {
			ai();
			await session.regenerate(false);
		},
		swipe: async () => {
			ai();
			await session.regenerate(true);
		},
		gen: async ({ unnamed }) => {
			ai();
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
		setvar: async ({ named, unnamed }) => {
			const key = text(named.key).trim();
			if (!key) throw new ScriptError("/setvar: key=이름 이 필요함");
			vars[key] = text(unnamed);
			await library.patchMeta(convId, { variables: { ...vars } });
		},
		getvar: ({ named, unnamed }) => vars[(text(named.key) || text(unnamed)).trim()] ?? "",
	};

	return {
		commands: commandsTable,
		unsupported: UNSUPPORTED,
		macro: (name, arg) => {
			switch (name) {
				case "char":
					return charName();
				case "user":
					return userName();
				case "getvar":
					return vars[(arg ?? "").trim()] ?? "";
				case "lastmessage":
					return session.messages.at(-1)?.content ?? "";
				case "newline":
					return "\n";
				default:
					return undefined;
			}
		},
	};
}

/** 입력칸·빠른 답장에서 실행. 오류는 알림으로 (멈춘 것은 조용히) */
export async function runSlash(source: string, session: ChatSession): Promise<void> {
	if (session.busy) return void toasts.error("응답을 받는 중이라 실행하지 않음");
	try {
		await runScript(source, slashEnv(session));
	} catch (e) {
		if (e instanceof ScriptError) toasts.error(e.message);
		// 다른 오류는 call이 이미 알림을 띄움
	}
}

/** 입력이 슬래시 커맨드인가 (`/`로 시작, `//`·`/ ` 같은 글은 아님) */
export function isSlash(text: string): boolean {
	return /^\/[A-Za-z]/.test(text.trimStart());
}
