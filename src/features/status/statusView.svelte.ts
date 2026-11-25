// 상태창 고정 자리의 화면 상태 (5-F-4-2, ADR-029) — 이 기기에만(localStorage), 동기화하지 않는다.
// 데이터(변수·메시지)와 섞지 않는다: 여기에는 "어디에 어떻게 보이나"만.
const KEY = "sc.status.view";

export interface FloatRect {
	x: number;
	y: number;
	w: number;
	h: number;
}

interface Saved {
	/** 데스크톱: 패널 대신 떠 있는 창에 */
	floating: boolean;
	/** 여러 상태창을 별개의 팝업창으로 각각 띄울지 */
	separateWindows: boolean;
	rect: FloatRect | null;
	/** 별도 창 모드에서 창별 위치·크기 (인덱스 키) */
	rects: Record<string, FloatRect>;
	/** 떠 있는 창을 접었나 */
	collapsed: boolean;
	/** 별도 창 모드에서 창별 접힘 상태 */
	collapsedMap: Record<string, boolean>;
	/** 채팅방 위 줄을 접었나 (떠 있는 창과 따로) */
	topCollapsed: boolean;
	/** 좁은 화면·Android: 채팅방 위에 늘 보이기 (사용자 요청, ADR-029 ⑤) */
	aboveChat: boolean;
}

const DEFAULTS: Saved = {
	floating: false,
	separateWindows: false,
	rect: null,
	rects: {},
	collapsed: false,
	collapsedMap: {},
	topCollapsed: false,
	aboveChat: false,
};

function load(): Saved {
	try {
		const raw = localStorage.getItem(KEY);
		const parsed = raw ? (JSON.parse(raw) as Partial<Saved>) : {};
		return {
			...DEFAULTS,
			...parsed,
			rects: parsed.rects ?? {},
			collapsedMap: parsed.collapsedMap ?? {},
		};
	} catch {
		return { ...DEFAULTS };
	}
}

class StatusView {
	#s = $state<Saved>(load());

	get floating() {
		return this.#s.floating;
	}
	get separateWindows() {
		return this.#s.separateWindows;
	}
	get rect() {
		return this.#s.rect;
	}
	get collapsed() {
		return this.#s.collapsed;
	}
	get topCollapsed() {
		return this.#s.topCollapsed;
	}
	get aboveChat() {
		return this.#s.aboveChat;
	}
	getRect(key: string): FloatRect | null {
		return this.#s.rects[key] ?? null;
	}
	setRect(key: string, rect: FloatRect) {
		this.set({ rects: { ...this.#s.rects, [key]: rect } });
	}
	isCollapsed(key: string): boolean {
		return !!this.#s.collapsedMap[key];
	}
	setCollapsed(key: string, collapsed: boolean) {
		this.set({ collapsedMap: { ...this.#s.collapsedMap, [key]: collapsed } });
	}
	set(patch: Partial<Saved>) {
		this.#s = { ...this.#s, ...patch };
		try {
			localStorage.setItem(KEY, JSON.stringify(this.#s));
		} catch {
			/* 저장 실패는 무시 (이번 실행 동안만) */
		}
	}
}

export const statusView = new StatusView();
