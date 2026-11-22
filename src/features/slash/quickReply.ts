// 빠른 답장 (5-E-20-5-3, ADR-025): 세트는 플러그인 설정(quickReply, 같이 씀), 대화마다 쓸 세트는 대화 meta.quickReplySets.
// 누르면: `/`로 시작하면 스크립트, 아니면 세트의 autoSend에 따라 보내기(AI 답장 요금) 또는 입력칸에 넣기.
import type { QuickReply_Serialize as QuickReply, QuickReplySet_Serialize as QuickReplySet } from "$ipc/bindings";
import { pluginSetting } from "$shared/stores/aiTools.svelte";
import { library, type Conversation } from "$features/character/library.svelte";
import type { ChatSession } from "$features/character/session.svelte";
import { isSlash, runSlash } from "./commands";

export function qrSets(): QuickReplySet[] {
	return (pluginSetting.value?.quickReply?.sets ?? []) as QuickReplySet[];
}

export async function saveSets(sets: QuickReplySet[]) {
	const cur = pluginSetting.value;
	if (!cur) return;
	await pluginSetting.save({ ...cur, quickReply: { sets } });
}

/** 이 대화가 고른 세트 id (없으면 null → 켜 둔 세트 전부) */
export function chosenIds(conv: Conversation | undefined): string[] | null {
	const ids = (conv?.meta as { quickReplySets?: unknown } | null)?.quickReplySets;
	return Array.isArray(ids) ? ids.filter((x): x is string => typeof x === "string") : null;
}

/** 이 대화의 버튼 줄에 보일 세트 */
export function setsFor(conv: Conversation | undefined): QuickReplySet[] {
	const ids = chosenIds(conv);
	return qrSets().filter((s) => (ids ? ids.includes(s.id) : s.enabled));
}

/** 대화에서 세트 켜고 끄기 (처음 바꾸면 지금 보이는 세트에서 시작) */
export async function toggleForConv(conv: Conversation, setId: string) {
	const ids = chosenIds(conv) ?? qrSets().filter((s) => s.enabled).map((s) => s.id);
	const next = ids.includes(setId) ? ids.filter((x) => x !== setId) : [...ids, setId];
	await library.patchMeta(conv.id, { quickReplySets: next });
}

/** 버튼을 눌렀을 때 */
export async function runQuickReply(q: QuickReply, set: QuickReplySet, session: ChatSession, draft: { get: () => string; set: (v: string) => void }) {
	if (isSlash(q.message)) return runSlash(q.message, session, { input: draft.get() });
	if (!set.autoSend) {
		const cur = draft.get();
		return draft.set(cur ? `${cur}\n${q.message}` : q.message);
	}
	if (session.busy) return;
	await session.send(q.message);
}

export const newId = () => (globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`);
