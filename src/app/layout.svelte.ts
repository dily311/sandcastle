// 앱 레이아웃 상태: 왼쪽 사이드바, 모바일(좁은 화면·터치) 여부
// - 데스크톱: 사이드바를 접고 펴며 이 컴퓨터에 기억

import { platform } from "@tauri-apps/plugin-os";

// - 모바일(폭 900px 이하: 휴대폰·좁은 창 — 대화 목록·패널을 겹쳐 열 자리만 있음): 사이드바는 위에 겹쳐 열리는 서랍, 기본은 닫힘
const KEY = "sc.sidebar";
export const MOBILE_QUERY = "(max-width: 900px)";
const PLATFORM = platform();

function load() {
	try {
		return localStorage.getItem(KEY) !== "0";
	} catch {
		return true;
	}
}
function media(q: string) {
	return typeof window !== "undefined" && typeof window.matchMedia === "function" ? window.matchMedia(q) : null;
}

class Layout {
	private pinned = $state(load());
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
	closeDrawer() {
		this.drawer = false;
	}
}

export const layout = new Layout();
