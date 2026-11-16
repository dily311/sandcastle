// 위젯 창의 테마: 앱 설정의 화면 모드·강조색·글자 크기를 위젯 화면 변수에 반영한다.
// (위젯은 예전 색 변수 --accent-color 등을 쓴다. 메인 앱 테마는 $app/theme.svelte.ts)
import type { ThemeSetting } from "$ipc/bindings";
import { commands } from "$ipc/bindings";
import { onEvent } from "$shared/ipc/events";
import { ACCENTS, DEFAULT_ACCENT, accentValue } from "$shared/ui/accents";
import { generateColorPalette } from "../utils/ContrastColorPalette";

export const defaultThemeSetting: ThemeSetting = {
	mode: "system",
	selectTheme: "default",
	isFrameless: true,
	accentColor: DEFAULT_ACCENT,
	fontSize: 16,
	grayTone: "slate",
	snippets: [],
	backgroundImage: null,
	backgroundOpacity: 0.82,
};

class ThemeStore {
	private setting = $state<ThemeSetting>(defaultThemeSetting);
	private mediaQuery: MediaQueryList | null = null;
	private listener: ((e: MediaQueryListEvent) => void) | null = null;

	get mode() {
		return this.setting.mode;
	}
	get isFrameless() {
		return this.setting.isFrameless;
	}
	get fontSize() {
		return this.setting.fontSize;
	}

	async init() {
		await onEvent<ThemeSetting>("theme_setting::changed", (c) => this.set(c));
		try {
			this.setting = (await commands.getThemeSetting()) as ThemeSetting;
		} catch {
			/* 기본값으로 */
		}
		this.updateDom();
	}

	set(changed: ThemeSetting) {
		this.setting = changed;
		this.updateDom();
	}

	private updateDom() {
		this.applyThemeToDom(this.setting);
		this.setupSystemWatcher(this.setting);
	}

	/** 설정의 강조색: 고정 팔레트 id(새 방식)면 그 색, 예전처럼 #rrggbb면 그대로 */
	private accentHex(isDark: boolean, accent: string) {
		if (/^#[0-9a-f]{6}$/i.test(accent)) return accent;
		return accentValue(ACCENTS.some((a) => a.id === accent) ? accent : DEFAULT_ACCENT, isDark);
	}

	private applyThemeToDom(theme: ThemeSetting) {
		const isSystemDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
		const isDark = theme.mode === "dark" || (theme.mode === "system" && isSystemDark);
		document.body.classList.toggle("is-frameless", !!theme.isFrameless);
		document.body.classList.toggle("theme-dark", isDark);
		document.body.classList.toggle("theme-light", !isDark);

		const palette = generateColorPalette(isDark, this.accentHex(isDark, theme.accentColor));
		document.documentElement.style.setProperty("font-size", `${theme.fontSize}px`);
		document.body.style.setProperty("--color-text", palette.textColor);
		document.body.style.setProperty("--color-bg-light", palette.bgLightColor);
		document.body.style.setProperty("--accent-color", palette.accentColor);
	}

	private setupSystemWatcher(theme: ThemeSetting) {
		if (this.mediaQuery && this.listener) this.mediaQuery.removeEventListener("change", this.listener);
		if (theme.mode === "system") {
			this.mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
			this.listener = () => this.applyThemeToDom(theme);
			this.mediaQuery.addEventListener("change", this.listener);
		}
	}
}

export const themeSetting = new ThemeStore();
