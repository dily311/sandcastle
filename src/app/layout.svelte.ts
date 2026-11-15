// 앱 레이아웃 상태: 왼쪽 사이드바, 모바일(좁은 화면·터치) 여부
// - 데스크톱: 사이드바를 접고 펴며 이 컴퓨터에 기억
// - 모바일(폭 760px 이하): 사이드바는 위에 겹쳐 열리는 서랍, 기본은 닫힘
const KEY = "sc.sidebar";
export const MOBILE_QUERY = "(max-width: 760px)";

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
	/** 좁은 화면 (휴대폰, 또는 아주 좁힌 창) */
	mobile = $state(media(MOBILE_QUERY)?.matches ?? false);
	/** 마우스 대신 손가락 (hover가 없음) */
	touch = $state(media("(hover: none)")?.matches ?? false);

	constructor() {
		media(MOBILE_QUERY)?.addEventListener("change", (e) => {
			this.mobile = e.matches;
			this.drawer = false;
		});
		media("(hover: none)")?.addEventListener("change", (e) => (this.touch = e.matches));
	}

	/** 사이드바가 보이는지 (모바일이면 서랍) */
	get sidebar() {
		return this.mobile ? this.drawer : this.pinned;
	}
	toggleSidebar() {
		if (this.mobile) {
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
