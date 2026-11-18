// 메시지 후처리 (CLAUDE.md 6-D-4, ADR-012). 새 AI 응답이 저장된 뒤(정규식까지 적용된 글) 플러그인을 설치 순서로 부른다.
// - 플러그인: sandcastle.on("message.process", (msg) => ({ content?, notes? }))
// - "자동 고침"(fix) 플러그인의 content만 받아 다음 플러그인에 넘긴다. "표시만"(notes)은 메모만
// - 결과는 백엔드가 저장하고(원문은 meta에), 그 사이 메시지가 바뀌었으면 저장하지 않는다
// - 스트리밍 중에는 부르지 않는다 (afterReply는 저장이 끝난 뒤)
import type { Message_Serialize as Message, PluginNote, PostProcessResult, RuntimePlugin } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { logs } from "$shared/stores/log.svelte";

export const PROCESS_EVENT = "message.process";
const PERMISSION = "messages.process";
const MAX_NOTES = 20;

type Emit = (pluginId: string, event: string, data: unknown) => Promise<unknown>;

function notesOf(v: unknown): PluginNote[] {
	if (!Array.isArray(v)) return [];
	return v
		.slice(0, MAX_NOTES)
		.map((n) => (typeof n === "string" ? { level: "info", text: n } : n && typeof n === "object" ? { level: (n as { level?: unknown }).level === "warn" ? "warn" : "info", text: String((n as { text?: unknown }).text ?? "") } : null))
		.filter((n): n is PluginNote => !!n && n.text.trim() !== "");
}

/** 플러그인들을 차례로 부른다 (순수 — 테스트한다). 실패·시간 초과한 플러그인은 건너뛴다 */
export async function chain(m: Pick<Message, "id" | "role" | "content" | "conversationId" | "name">, plugins: readonly RuntimePlugin[], emit: Emit) {
	let content = m.content;
	const results: PostProcessResult[] = [];
	const errors: { pluginId: string; message: string }[] = [];
	for (const p of plugins) {
		if (!p.granted.includes(PERMISSION)) continue;
		let out: unknown;
		try {
			out = await emit(p.id, PROCESS_EVENT, { message: { id: m.id, role: m.role, name: m.name ?? null, conversationId: m.conversationId, content } });
		} catch (e) {
			errors.push({ pluginId: p.id, message: e instanceof Error ? e.message : String(e) });
			continue;
		}
		if (out === null || out === undefined) continue;
		if (typeof out !== "object") continue;
		const o = out as { content?: unknown; notes?: unknown };
		const notes = notesOf(o.notes);
		const next = typeof o.content === "string" ? o.content : null;
		const changed = p.processMode === "fix" && next !== null && next !== content;
		if (changed) content = next;
		if (changed || notes.length) results.push({ pluginId: p.id, changed, notes });
	}
	return { content, results, errors };
}

/** afterReply 훅: 고쳐진 메시지를 돌려준다 (아무것도 없으면 그대로) */
export async function postProcess(m: Message, plugins: readonly RuntimePlugin[], emit: Emit): Promise<Message> {
	if (m.role !== "assistant" || !plugins.some((p) => p.granted.includes(PERMISSION))) return m;
	const { content, results, errors } = await chain(m, plugins, emit);
	for (const e of errors) logs.warn(`plugin:${e.pluginId}`, `후처리 실패: ${e.message}`);
	if (!results.length) return m;
	return call(commands.applyPluginPostprocess(m.id, m.content, content, results), { source: "plugins" }).catch(() => m);
}

export type PluginProcessMeta = {
	original: string;
	content: string;
	changedBy: string[];
	notes: Record<string, PluginNote[]>;
	at: number;
};

export function pluginProcessOf(m: Message): PluginProcessMeta | null {
	const p = (m.meta as { pluginProcess?: PluginProcessMeta } | null)?.pluginProcess;
	return p && typeof p === "object" ? p : null;
}
