// 읽던 자리 (5-E-6-2-3, ADR-031): 대화마다 화면 위쪽에 걸린 메시지와 그 안에서 내려온 비율.
// 픽셀이 아니라 메시지 기준이라 글꼴·창 크기가 바뀌어도 맞다. 이 기기의 화면 상태라 브라우저 저장소에만 (동기화하지 않음)

export interface ReadPos {
	messageId: string;
	/** 그 메시지 높이 중 화면 위쪽 위로 넘어간 몫 (0~1) */
	ratio: number;
}

const KEY = "sandcastle.readPos";
/** 대화 수가 많아도 저장소가 끝없이 늘지 않게, 최근에 읽은 것만 */
export const MAX_KEPT = 200;

type Store = Record<string, ReadPos & { at: number }>;

function read(): Store {
	try {
		const v = JSON.parse(localStorage.getItem(KEY) ?? "{}");
		return v && typeof v === "object" ? (v as Store) : {};
	} catch {
		return {};
	}
}

export function getReadPos(convId: string): ReadPos | null {
	const p = read()[convId];
	return p && typeof p.messageId === "string" ? { messageId: p.messageId, ratio: Math.min(Math.max(Number(p.ratio) || 0, 0), 1) } : null;
}

export function setReadPos(convId: string, pos: ReadPos, now = Date.now()) {
	const all = read();
	all[convId] = { ...pos, at: now };
	const ids = Object.keys(all);
	if (ids.length > MAX_KEPT) {
		ids.sort((a, b) => all[a].at - all[b].at);
		for (const id of ids.slice(0, ids.length - MAX_KEPT)) delete all[id];
	}
	try {
		localStorage.setItem(KEY, JSON.stringify(all));
	} catch {
		/* 저장소를 못 쓰면 기억하지 않음 */
	}
}

/** 스크롤 칸 위쪽에 걸린 메시지 (id가 msg-로 시작하는 칸) */
export function anchorIn(scroller: HTMLElement): ReadPos | null {
	const top = scroller.getBoundingClientRect().top;
	for (const el of scroller.querySelectorAll<HTMLElement>("[id^=msg-]")) {
		const r = el.getBoundingClientRect();
		if (r.bottom > top + 1) return { messageId: el.id.slice(4), ratio: r.height ? Math.min(Math.max((top - r.top) / r.height, 0), 1) : 0 };
	}
	return null;
}

/** 그 메시지의 그 몫이 화면 위쪽에 오게 */
export function scrollToPos(scroller: HTMLElement, pos: ReadPos): boolean {
	const el = scroller.querySelector<HTMLElement>(`#msg-${CSS.escape(pos.messageId)}`);
	if (!el) return false;
	const r = el.getBoundingClientRect();
	scroller.scrollTop += r.top - scroller.getBoundingClientRect().top + pos.ratio * r.height;
	return true;
}

/** 스크롤 칸 가운데 줄에 걸린 메시지 (읽기 화면의 "지금 읽는 메시지", 5-E-6-1) */
export function messageAtMiddle(scroller: HTMLElement): string | null {
	const all = scroller.querySelectorAll<HTMLElement>("[id^=msg-]");
	if (!all.length) return null;
	// 끝까지 내렸으면 마지막, 맨 위면 첫 메시지 (짧은 메시지가 가운데 줄에 닿지 못해도 고를 수 있게)
	if (scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 2) return all[all.length - 1].id.slice(4);
	if (scroller.scrollTop <= 0) return all[0].id.slice(4);
	const r = scroller.getBoundingClientRect();
	const mid = r.top + r.height / 2;
	let last: string | null = null;
	for (const el of all) {
		const b = el.getBoundingClientRect();
		if (b.top > mid) break;
		last = el.id.slice(4);
	}
	return last;
}
