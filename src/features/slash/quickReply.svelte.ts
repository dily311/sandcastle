// 빠른 답장 (5-E-20-5-3·4, ADR-025): 세트는 DB(quick_reply_sets), 정규식처럼 전역·프리셋·캐릭터 전용.
// 대화에 보이는 세트 = 켜 둔 세트 중 전역 + 이 대화 캐릭터 전용 + 이 대화 프리셋 전용, 대화에서 끈 것(meta.quickReplyOff)은 뺌.
// 누르면: `/`로 시작하면 스크립트, 아니면 세트의 autoSend에 따라 보내기(AI 답장 요금) 또는 입력칸에 넣기.
import type { QuickReplySetInput, QuickReplySet_Serialize as QuickReplySet, QuickReply_Serialize as QuickReply } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { library, type Conversation } from "$features/character/library.svelte";
import type { ChatSession } from "$features/character/session.svelte";
import { isSlash, runSlash } from "./commands";
import { presetOf } from "$features/character/presetMacros.svelte";

export type { QuickReply, QuickReplySet };
export type Scope = "global" | "character" | "preset";
const src = { source: "slash" };

class QuickReplyStore {
	sets = $state<QuickReplySet[]>([]);
	loaded = $state(false);
	/** 빠른 답장 버튼 줄 숨기기 (번개 아이콘 메뉴에서 전환, 기기 저장소에 영구 저장) */
	barHidden = $state<boolean>(
		(() => {
			try {
				return localStorage.getItem("sc.qr.bar.hidden") === "1";
			} catch {
				return false;
			}
		})(),
	);
	private loading: Promise<void> | null = null;

	setBarHidden(v: boolean) {
		this.barHidden = v;
		try {
			localStorage.setItem("sc.qr.bar.hidden", v ? "1" : "0");
		} catch {
			/* 저장소 접근 불가 시 이번 실행 동안만 유지 */
		}
	}

	load(force = false) {
		if (this.loaded && !force) return Promise.resolve();
		this.loading ??= call(commands.listQuickReplySets(), src)
			.then((list) => {
				this.sets = list;
				this.loaded = true;
			})
			.finally(() => (this.loading = null));
		return this.loading;
	}
	async save(input: QuickReplySetInput) {
		const saved = await call(commands.upsertQuickReplySet(input), src);
		const i = this.sets.findIndex((s) => s.id === saved.id);
		if (i >= 0) this.sets[i] = saved;
		else this.sets.push(saved);
		return saved;
	}
	async remove(id: string) {
		await call(commands.deleteQuickReplySet(id), src);
		this.sets = this.sets.filter((s) => s.id !== id);
	}
}
export const quickReplies = new QuickReplyStore();

/** 범위로 이 대화에 해당하는 세트 (켜짐·대화에서 끈 것과 상관없이) */
export function setsInScope(conv: Conversation | undefined, sets: QuickReplySet[] = quickReplies.sets): QuickReplySet[] {
	if (!conv) return sets.filter((s) => s.scope === "global");
	const preset = presetOf(conv);
	return sets.filter((s) => s.scope === "global" || (s.scope === "character" && s.scopeId === conv.characterId) || (s.scope === "preset" && !!preset && s.scopeId === preset));
}

/** 이 대화에서 끈 세트 id */
export function offIds(conv: Conversation | undefined): string[] {
	const ids = (conv?.meta as { quickReplyOff?: unknown } | null)?.quickReplyOff;
	return Array.isArray(ids) ? ids.filter((x): x is string => typeof x === "string") : [];
}

/** 이 대화의 버튼에 쓰일 세트 */
export function setsFor(conv: Conversation | undefined, sets: QuickReplySet[] = quickReplies.sets): QuickReplySet[] {
	const off = offIds(conv);
	return setsInScope(conv, sets).filter((s) => s.enabled && !off.includes(s.id));
}

/** 대화에서 세트 끄고 켜기 */
export async function toggleForConv(conv: Conversation, setId: string) {
	const off = offIds(conv);
	await library.patchMeta(conv.id, { quickReplyOff: off.includes(setId) ? off.filter((x) => x !== setId) : [...off, setId] });
}

/** 버튼 하나와 그 세트 (검색·모두 보기용) */
export interface QuickButton {
	set: QuickReplySet;
	q: QuickReply;
}
export function buttonsOf(sets: QuickReplySet[], query = ""): QuickButton[] {
	const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
	return sets
		.flatMap((set) => set.items.filter((q) => !q.hidden && (q.label.trim() || q.message.trim())).map((q) => ({ set, q })))
		.filter(({ set, q }) => words.every((w) => `${q.label} ${q.message} ${set.name}`.toLowerCase().includes(w)));
}

/** 버튼을 눌렀을 때 */
export async function runQuickReply(q: QuickReply, set: QuickReplySet, session: ChatSession, draft: { get: () => string; set: (v: string) => void }) {
	if (isSlash(q.message)) return runSlash(q.message, session, { input: draft.get(), setInput: draft.set });
	if (!set.autoSend) {
		const cur = draft.get();
		return draft.set(cur ? `${cur}\n${q.message}` : q.message);
	}
	if (session.busy) return;
	await session.send(q.message);
}

export const newId = () => globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`;

/** 세트 → 저장 입력 */
export function toInput(s: QuickReplySet): QuickReplySetInput {
	return { id: s.id, name: s.name, scope: s.scope, scopeId: s.scopeId, enabled: s.enabled, autoSend: s.autoSend, items: s.items, sortOrder: s.sortOrder };
}
