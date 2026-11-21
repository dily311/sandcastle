// 앱 전체 화면 이동 기록 (타이틀바 앞으로/뒤로, Alt+←/→, 마우스 옆 버튼)
export type Route =
	| { page: "home" }
	| { page: "phone" }
	| { page: "character"; charId?: string; convId?: string; edit?: boolean }
	| { page: "game"; gameId?: string; sessionId?: string; edit?: boolean }
	| { page: "persona"; personaId?: string }
	| { page: "lorebook"; bookId?: string }
	| { page: "images" }
	| { page: "settings"; tab?: SettingsTab; /** 테마 편집: 파일 이름, 또는 "new" / "builtin:id"(복제해서 새로). API 편집: 연결 id 또는 "new" */ edit?: string };

export type SettingsTab = "general" | "plugins" | "chat" | "phone" | "theme" | "shortcut" | "widget" | "api" | "preset" | "regex" | "data" | "log" | "activity" | "game" | "cg" | "translate" | "tts" | "imagegen" | "sync" | "community" | "lint" | "memory" | "summary" | "rewrite" | "ghostwrite" | `community:${string}`;
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

	/** 저장하지 않은 편집이 있는 화면이 등록: false를 돌려주면 이동을 막음 */
	private guard: (() => Promise<boolean>) | null = null;
	/** 화면을 떠나기 전에 물어볼 함수 등록. 돌려준 함수로 해제 */
	setGuard(fn: () => Promise<boolean>) {
		this.guard = fn;
		return () => {
			if (this.guard === fn) this.guard = null;
		};
	}
	/** 저장했거나 이미 물어봤을 때: 묻지 않고 떠남 */
	clearGuard() {
		this.guard = null;
	}
	private async leave() {
		if (!this.guard) return true;
		const ok = await this.guard();
		if (ok) this.guard = null;
		return ok;
	}

	/** 새 화면으로 이동. 같은 화면이면 무시, 앞으로 기록은 버림 */
	async go(route: Route) {
		if (JSON.stringify(route) === JSON.stringify(this.current)) return;
		if (!(await this.leave())) return;
		const next = [...this.stack.slice(0, this.index + 1), route].slice(-MAX);
		this.stack = next;
		this.index = next.length - 1;
	}
	/** 기록을 남기지 않고 현재 위치만 바꿈 (탭 전환 등) */
	async replace(route: Route) {
		if (!(await this.leave())) return;
		this.stack[this.index] = route;
	}
	async back() {
		if (this.canBack && (await this.leave())) this.index--;
	}
	async forward() {
		if (this.canForward && (await this.leave())) this.index++;
	}
}

export const nav = new Nav();

// ---- 마지막으로 본 화면 (5-E-5) — 이 기기의 화면 상태라 브라우저 저장소에 ----
const LAST_KEY = "sandcastle.lastRoute";
const PAGES: Page[] = ["home", "phone", "character", "game", "persona", "lorebook", "images", "settings"];

/** 다시 열 때 쓸 모양: 편집 화면·게임 플레이는 열지 않는다 (편집기는 저장하지 않은 것처럼 보이고, 플레이는 소리·작업을 시작함) */
export function restorable(r: Route): Route {
	const { edit: _e, ...rest } = r as Route & { edit?: unknown };
	if (rest.page === "game") return { page: "game", gameId: rest.gameId };
	return rest as Route;
}

export function saveLastRoute(r: Route) {
	try {
		localStorage.setItem(LAST_KEY, JSON.stringify(restorable(r)));
	} catch {
		/* 저장소를 못 쓰면 다음엔 기본 화면 */
	}
}

export function lastRoute(): Route | null {
	try {
		const r = JSON.parse(localStorage.getItem(LAST_KEY) ?? "null") as Route | null;
		return r && PAGES.includes(r.page) ? restorable(r) : null;
	} catch {
		return null;
	}
}
