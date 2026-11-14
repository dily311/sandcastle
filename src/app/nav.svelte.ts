// 앱 전체 화면 이동 기록 (타이틀바 앞으로/뒤로, Alt+←/→, 마우스 옆 버튼)
export type Route =
	| { page: "phone" }
	| { page: "character"; charId?: string; convId?: string }
	| { page: "game"; gameId?: string; sessionId?: string; edit?: boolean }
	| { page: "persona"; personaId?: string }
	| { page: "settings"; tab?: SettingsTab };

export type SettingsTab = "general" | "theme" | "shortcut" | "widget" | "api" | "preset" | "regex" | "data" | "log";
export type Page = Route["page"];

const MAX = 100;

class Nav {
	stack = $state<Route[]>([{ page: "character" }]);
	index = $state(0);

	get current(): Route {
		return this.stack[this.index];
	}
	get canBack() {
		return this.index > 0;
	}
	get canForward() {
		return this.index < this.stack.length - 1;
	}

	/** 새 화면으로 이동. 같은 화면이면 무시, 앞으로 기록은 버림 */
	go(route: Route) {
		if (JSON.stringify(route) === JSON.stringify(this.current)) return;
		const next = [...this.stack.slice(0, this.index + 1), route].slice(-MAX);
		this.stack = next;
		this.index = next.length - 1;
	}
	/** 기록을 남기지 않고 현재 위치만 바꿈 (탭 전환 등) */
	replace(route: Route) {
		this.stack[this.index] = route;
	}
	back() {
		if (this.canBack) this.index--;
	}
	forward() {
		if (this.canForward) this.index++;
	}
}

export const nav = new Nav();
