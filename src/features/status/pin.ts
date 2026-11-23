// 상태창 고정 (5-F-4-2, ADR-029): 메시지 속 HTML 블록에 `data-sc-pin="status"`가 있으면 상태창이다.
// 메시지에서는 접고, 고정 자리(상태 탭·띄운 창·채팅방 위)에는 출처마다 가장 최근 것 하나씩 그린다 (5-F-5-2).
// 출처 = 표시 값: "status"(손으로 붙인 것), "status:<정규식 id>"(상태창 정규식, 5-F-5-1), "status:이름"
import { toBlocks, type Block } from "$features/character/format";

/** 표시 (SillyTavern 정규식 결과·카드의 상태창 HTML에 붙인다) */
export const PIN_ATTR = 'data-sc-pin="status"';
const PIN_RE = /\bdata-sc-pin\s*=\s*["']?status\b/i;

/** 이 HTML 블록이 상태창인가 */
export function isStatusPin(b: Block): b is Extract<Block, { kind: "html" }> {
	return b.kind === "html" && PIN_RE.test(b.code);
}

/** 뒤에서부터 이만큼만 본다 (긴 대화에서 매번 다 읽지 않게) */
const LOOKBACK = 60;

type Msg = { id: string; role: string; content: string };

/** 블록의 출처 (표시 값). 예: `status`, `status:r12` */
export function pinKey(b: Extract<Block, { kind: "html" }>): string {
	return /\bdata-sc-pin\s*=\s*["']?(status(?::[^"'\s>]*)?)/i.exec(b.code)?.[1] ?? "status";
}

/**
 * 출처마다 가장 최근 상태창 블록 (출처 이름순 — 순서가 메시지마다 바뀌지 않게). AI 메시지에서만 찾고,
 * 보이는 글(정규식을 거친 display)을 메시지와 같은 방식으로 블록으로 나눈다. 한 메시지에 같은 출처가 여럿이면 마지막 것
 */
export function latestStatusBlocks(messages: readonly Msg[], display: Record<string, string>, safeHtml: boolean): Extract<Block, { kind: "html" }>[] {
	const found = new Map<string, Extract<Block, { kind: "html" }>>();
	for (let i = messages.length - 1, n = 0; i >= 0 && n < LOOKBACK; i--) {
		const m = messages[i];
		if (m.role !== "assistant") continue;
		n++;
		const text = display[m.id] ?? m.content;
		if (!PIN_RE.test(text)) continue;
		const pinned = toBlocks(text, { safeHtml }).filter(isStatusPin);
		for (const b of pinned.reverse()) {
			const k = pinKey(b);
			if (!found.has(k)) found.set(k, b);
		}
	}
	return [...found.entries()].sort(([a], [b]) => a.localeCompare(b)).map(([, b]) => b);
}

/** 가장 최근 하나 (출처와 상관없이) */
export function latestStatusBlock(messages: readonly Msg[], display: Record<string, string>, safeHtml: boolean): Extract<Block, { kind: "html" }> | null {
	for (let i = messages.length - 1, n = 0; i >= 0 && n < LOOKBACK; i--) {
		const m = messages[i];
		if (m.role !== "assistant") continue;
		n++;
		const text = display[m.id] ?? m.content;
		if (!PIN_RE.test(text)) continue;
		const pinned = toBlocks(text, { safeHtml }).filter(isStatusPin);
		if (pinned.length) return pinned[pinned.length - 1];
	}
	return null;
}
