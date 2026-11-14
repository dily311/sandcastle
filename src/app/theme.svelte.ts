// 테마: 화면 모드(시스템/밝게/어둡게), 고정 강조색, 글자 크기, 사용자 CSS 테마
import type { ThemeSetting } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ACCENTS, DEFAULT_ACCENT, accentValue } from "$shared/ui/accents";

const FALLBACK: ThemeSetting = {
	mode: "system",
	selectTheme: "default",
	isFrameless: true,
	accentColor: DEFAULT_ACCENT,
	fontSize: 16,
};

class Theme {
	setting = $state<ThemeSetting>(FALLBACK);
	systemDark = $state(false);

	get dark() {
		return this.setting.mode === "dark" || (this.setting.mode === "system" && this.systemDark);
	}
	/** 예전 버전은 accentColor에 "#rrggbb"를 저장했다 → 고정 팔레트에 없으면 기본색 */
	get accentId() {
		return ACCENTS.some((a) => a.id === this.setting.accentColor) ? this.setting.accentColor : DEFAULT_ACCENT;
	}

	async init() {
		const mq = window.matchMedia("(prefers-color-scheme: dark)");
		this.systemDark = mq.matches;
		mq.addEventListener("change", (e) => {
			this.systemDark = e.matches;
			this.apply();
		});
		this.setting = await call(commands.getThemeSetting(), { source: "theme", silent: true }).catch(() => FALLBACK);
		this.apply();
		this.applyFrame();
	}

	async update(patch: Partial<ThemeSetting>) {
		this.setting = { ...this.setting, ...patch };
		this.apply();
		if ("isFrameless" in patch) this.applyFrame();
		await call(commands.updateThemeSetting(patch), { source: "theme" });
	}

	apply() {
		const root = document.documentElement;
		root.dataset.theme = this.dark ? "dark" : "light";
		root.style.setProperty("--accent", accentValue(this.accentId, this.dark));
		root.style.fontSize = `${this.setting.fontSize}px`;
		void this.applyCustomCss();
	}

	/** 프레임 없는 창이면 앱 헤더가 창 버튼을 그림, 아니면 운영체제 제목 표시줄 */
	private applyFrame() {
		if (!__IS_TAURI__) return;
		getCurrentWindow()
			.setDecorations(!this.setting.isFrameless)
			.catch(() => {});
	}

	private styleEl: HTMLStyleElement | null = null;
	private async applyCustomCss() {
		const name = this.setting.selectTheme;
		if (!this.styleEl) {
			this.styleEl = document.createElement("style");
			this.styleEl.id = "user-theme";
			document.head.appendChild(this.styleEl);
		}
		if (!name || name === "default") {
			this.styleEl.textContent = "";
			return;
		}
		this.styleEl.textContent = await call(commands.loadThemeContent(name), { source: "theme", silent: true }).catch(() => "");
	}
}

export const theme = new Theme();
