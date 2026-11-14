// 앱 왼쪽 사이드바 열림 상태 (이 컴퓨터에만 저장)
const KEY = "sc.sidebar";

function load() {
	try {
		return localStorage.getItem(KEY) !== "0";
	} catch {
		return true;
	}
}

class Layout {
	sidebar = $state(load());
	toggleSidebar() {
		this.sidebar = !this.sidebar;
		try {
			localStorage.setItem(KEY, this.sidebar ? "1" : "0");
		} catch {
			/* 저장 못 해도 동작 */
		}
	}
}

export const layout = new Layout();
