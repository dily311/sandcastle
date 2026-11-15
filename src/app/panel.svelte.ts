// 오른쪽 패널: 탭 아이콘은 타이틀바(패널 바로 위)에 그려지고, 화면이 자기 탭을 등록한다.
//   const off = panel.register([{ id: "settings", label: "대화 설정", icon: SlidersHorizontal }, ...]);
//   onDestroy(off)  — 또는 $effect 안에서 register를 부르고 반환값을 그대로 돌려주기
import type { Component } from "svelte";
import { layout } from "./layout.svelte";

export interface PanelTab {
	id: string;
	label: string;
	icon: Component;
}

const OPEN_KEY = "sc.panel.open";
/** 탭 순서: 화면마다(같은 탭 묶음마다) 사용자가 옮긴 순서를 기억한다 */
const ORDER_KEY = "sc.panel.order:";

/** 탭 묶음을 가리키는 키 (순서와 상관없이 같은 탭들이면 같은 키) */
export function tabSetKey(tabs: { id: string }[]): string {
	return tabs.map((t) => t.id).sort().join(",");
}

/** 저장된 순서대로 탭을 늘어놓는다. 저장에 없는 새 탭은 원래 자리 순서대로 뒤에 붙인다 */
export function applyOrder<T extends { id: string }>(tabs: T[], order: string[] | null): T[] {
	if (!order?.length) return tabs;
	const rank = new Map(order.map((id, i) => [id, i]));
	return [...tabs].sort((a, b) => (rank.get(a.id) ?? order.length + tabs.indexOf(a)) - (rank.get(b.id) ?? order.length + tabs.indexOf(b)));
}

/** from 탭을 to 탭 자리로 옮긴 순서 */
export function moveId(ids: string[], from: string, to: string): string[] {
	const i = ids.indexOf(from);
	const j = ids.indexOf(to);
	if (i < 0 || j < 0 || i === j) return ids;
	const out = [...ids];
	out.splice(i, 1);
	out.splice(j, 0, from);
	return out;
}

function loadOrder(key: string): string[] | null {
	try {
		const raw = localStorage.getItem(ORDER_KEY + key);
		const v = raw ? JSON.parse(raw) : null;
		return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : null;
	} catch {
		return null;
	}
}
const WIDTH_KEY = "sc.panel.width";
export const PANEL_MIN = 260;
export const PANEL_MAX = 560;

function loadWidth(): number {
	try {
		const n = Number(localStorage.getItem(WIDTH_KEY));
		return n >= PANEL_MIN && n <= PANEL_MAX ? n : 300;
	} catch {
		return 300;
	}
}

function loadOpen(): boolean {
	try {
		return localStorage.getItem(OPEN_KEY) !== "0";
	} catch {
		return true;
	}
}

class Panel {
	tabs = $state<PanelTab[]>([]);
	tab = $state("");
	/** 데스크톱에서 패널을 펴 두었는지 (기억함) */
	private pinned = $state(loadOpen());
	/** 모바일: 패널은 전체 화면 시트, 기본은 닫힘 (기억하지 않음) */
	private sheet = $state(false);
	/** 지금 패널이 보이는지 */
	get open() {
		return layout.mobile ? this.sheet : this.pinned;
	}
	width = $state(loadWidth());

	constructor() {
		this.applyWidth();
	}
	/** --panel-width를 바꿈 (대화 화면 칸과 타이틀바 탭 줄이 같이 따라감) */
	setWidth(w: number, save = false) {
		this.width = Math.round(Math.min(PANEL_MAX, Math.max(PANEL_MIN, w)));
		this.applyWidth();
		if (save) {
			try {
				localStorage.setItem(WIDTH_KEY, String(this.width));
			} catch {
				/* 저장 실패는 무시 */
			}
		}
	}
	private applyWidth() {
		if (typeof document !== "undefined") document.documentElement.style.setProperty("--panel-width", `${this.width}px`);
	}

	get available() {
		return this.tabs.length > 0;
	}

	/** 지금 탭을 등록한 화면 (해제할 때 다른 화면의 등록을 지우지 않게) */
	private owner = 0;

	register(tabs: PanelTab[], initial = tabs[0]?.id ?? "") {
		// $state에 넣으면 배열이 프록시로 바뀌어 원래 배열과 비교할 수 없음 → 번호표로 확인
		const me = ++this.owner;
		this.tabs = applyOrder(tabs, loadOrder(tabSetKey(tabs)));
		if (!tabs.some((t) => t.id === this.tab)) this.tab = initial;
		return () => {
			if (this.owner === me) this.tabs = [];
		};
	}

	// ---- 탭 순서 바꾸기: 끌어다 놓기, 또는 탭에서 Alt+←/→ ----
	private dragging: string | null = null;

	/** 탭 버튼에 펼쳐 넣는 속성: {...panel.tabDrag(t.id)} */
	tabDrag(id: string) {
		return {
			draggable: true,
			ondragstart: (e: DragEvent) => {
				this.dragging = id;
				e.dataTransfer?.setData("text/plain", id);
				if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
			},
			ondragover: (e: DragEvent) => {
				if (this.dragging && this.dragging !== id) e.preventDefault();
			},
			ondrop: (e: DragEvent) => {
				e.preventDefault();
				if (this.dragging) this.moveTab(this.dragging, id);
				this.dragging = null;
			},
			ondragend: () => (this.dragging = null),
			onkeydown: (e: KeyboardEvent) => {
				if (!e.altKey || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) return;
				e.preventDefault();
				e.stopPropagation(); // 앞으로·뒤로 가기(Alt+←/→)와 겹치지 않게
				const ids = this.tabs.map((t) => t.id);
				const target = ids[ids.indexOf(id) + (e.key === "ArrowLeft" ? -1 : 1)];
				if (target) this.moveTab(id, target);
			},
		};
	}

	moveTab(from: string, to: string) {
		const ids = moveId(
			this.tabs.map((t) => t.id),
			from,
			to,
		);
		this.tabs = applyOrder(this.tabs, ids);
		try {
			localStorage.setItem(ORDER_KEY + tabSetKey(this.tabs), JSON.stringify(ids));
		} catch {
			/* 저장 실패는 무시 (이번 실행 동안만 유지) */
		}
	}

	toggle() {
		if (layout.mobile) {
			this.sheet = !this.sheet;
			return;
		}
		this.pinned = !this.pinned;
		try {
			localStorage.setItem(OPEN_KEY, this.pinned ? "1" : "0");
		} catch {
			/* 저장 실패는 무시 */
		}
	}
	/** 모바일 시트 닫기 (다른 화면으로 갈 때) */
	closeSheet() {
		this.sheet = false;
	}
}

export const panel = new Panel();
