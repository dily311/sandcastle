// 앱 레이아웃 상태: 왼쪽 사이드바, 모바일(좁은 화면·터치) 여부
// - 데스크톱: 사이드바를 접고 펴며 이 컴퓨터에 기억

import { platform } from "@tauri-apps/plugin-os";

// - 모바일(폭 900px 이하: 휴대폰·좁은 창 — 대화 목록·패널을 겹쳐 열 자리만 있음): 사이드바는 위에 겹쳐 열리는 서랍, 기본은 닫힘
const KEY = "sc.sidebar";
const WIDTH_KEY = "sc.sidebar.width";
export const SIDEBAR_MIN = 180;
export const SIDEBAR_MAX = 480;
const SIDEBAR_DEFAULT = 220;
export const MOBILE_QUERY = "(max-width: 900px)";
const PLATFORM = platform();

function load() {
	try {
		return localStorage.getItem(KEY) !== "0";
	} catch {
		return true;
	}
}
function loadWidth() {
	try {
		const n = Number(localStorage.getItem(WIDTH_KEY));
		return Number.isFinite(n) && n >= SIDEBAR_MIN && n <= SIDEBAR_MAX ? n : SIDEBAR_DEFAULT;
	} catch {
		return SIDEBAR_DEFAULT;
	}
}
function media(q: string) {
	return typeof window !== "undefined" && typeof window.matchMedia === "function" ? window.matchMedia(q) : null;
}

class Layout {
	private pinned = $state(load());
	/** 데스크톱 사이드바 너비 (끌어서 바꿈, 이 컴퓨터에 기억) */
	sidebarWidth = $state(loadWidth());
	/** 모바일 서랍 열림 */
	drawer = $state(false);
	/** 휴대폰만 적용 */
	mobile = $state(PLATFORM === "android" || PLATFORM === "ios");
	/** 좁은 창인지 판단 */
	narrow = $state(media(MOBILE_QUERY)?.matches ?? false);
	/** 마우스 대신 손가락 (hover가 없음) */
	/** 데스크톱 앱인가 (창 크기와 무관). 좁은 창이라 모바일 레이아웃이어도 창 버튼은 데스크톱이면 보여야 한다 */
	readonly desktop = typeof navigator === "undefined" || !/Android|iPhone|iPad|iPod/i.test(navigator.userAgent);
	touch = $state(media("(hover: none)")?.matches ?? false);

	constructor() {
		media(MOBILE_QUERY)?.addEventListener("change", (e) => {
			this.narrow = e.matches;
			this.drawer = false;
		});
		media("(hover: none)")?.addEventListener("change", (e) => (this.touch = e.matches));
	}

	/** 사이드바가 보이는지 (모바일이면 서랍) */
	get sidebar() {
		return this.narrow ? this.drawer : this.pinned;
	}
	toggleSidebar() {
		if (this.narrow) {
			this.drawer = !this.drawer;
			return;
		}
		this.pinned = !this.pinned;
		try {
			localStorage.setItem(KEY, this.pinned ? "1" : "0");
		} catch {
			/* 저장 못 해도 동작 */
		}
	}
	/** 사이드바 너비. save가 true면(끌기를 놓을 때) 기억한다 */
	setSidebarWidth(w: number, save = false) {
		this.sidebarWidth = Math.round(Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, w)));
		if (!save) return;
		try {
			localStorage.setItem(WIDTH_KEY, String(this.sidebarWidth));
		} catch {
			/* 저장 못 해도 동작 */
		}
	}
	closeDrawer() {
		this.drawer = false;
	}
}

export const layout = new Layout();
