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
