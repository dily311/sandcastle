// 오른쪽 패널: 탭 아이콘은 타이틀바(패널 바로 위)에 그려지고, 화면이 자기 탭을 등록한다.
//   const off = panel.register([{ id: "settings", label: "대화 설정", icon: SlidersHorizontal }, ...]);
//   onDestroy(off)  — 또는 $effect 안에서 register를 부르고 반환값을 그대로 돌려주기
import type { Component } from "svelte";

export interface PanelTab {
	id: string;
	label: string;
	icon: Component;
}

const OPEN_KEY = "sc.panel.open";
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
	open = $state(loadOpen());
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
		this.tabs = tabs;
		if (!tabs.some((t) => t.id === this.tab)) this.tab = initial;
		return () => {
			if (this.owner === me) this.tabs = [];
		};
	}

	toggle() {
		this.open = !this.open;
		try {
			localStorage.setItem(OPEN_KEY, this.open ? "1" : "0");
		} catch {
			/* 저장 실패는 무시 */
		}
	}
}

export const panel = new Panel();
