import type { CssTheme, ThemeSetting } from "$/lib/type/settingTypes";
import { themeCommands } from "$ipc/commands";
import { generateColorPalette } from "../utils/ContrastColorPalette";

export const defaultThemeSetting: ThemeSetting = {
	mode: "system",
	selectTheme: "default",
	isFrameless: true,
	accentColor: "#8b85f9",
	fontSize: 16,
};

class ThemeStore {
	private setting = $state<ThemeSetting>(defaultThemeSetting);
	private cssThemes = $state<CssTheme[]>([]);

	private mediaQuery: MediaQueryList | null = null;
	private listener: ((e: MediaQueryListEvent) => void) | null = null;
	private styleTag = document.head.querySelector("#custom-theme-style");

	// =======================================
	// get
	// =======================================
	get mode() {return this.setting.mode};
	get selectTheme() {return this.setting.selectTheme};
	get isFrameless() {return this.setting.isFrameless};
	get accentColor() {return this.setting.accentColor};
	get fontSize() {return this.setting.fontSize};
	get cssThemeValue() {return this.cssThemes};

	
	// =======================================
	// init
	// =======================================
	async init() {
		// 리스너 등록
		await themeCommands.listen_themeSettingsChanged((c) => this.set(c));
		// 설정 불러오고 적용
		const setting = await themeCommands.getThemeSettings();
		const cssThemeList = await themeCommands.getThemeList();
		this.setting = setting;
		this.cssThemes = cssThemeList;
		// dom에 반영
		this.updateDom();
	}

	// =======================================
	// 업데이트용 함수
	// =======================================
	set(changed: ThemeSetting) {
		if (this.isUpdate(changed)) return;
		this.setting = changed;
		// dom에 반영
		this.updateDom();
	}

	isUpdate(patch: Partial<ThemeSetting>) {
		const isIdentical = Object.entries(patch).every(
			([key, value]) => this.setting[key as keyof ThemeSetting] === value,
		);
		return isIdentical;
	}

	async update(changed: Partial<ThemeSetting>) {
		if (this.isUpdate(changed)) return;

		this.setting = { ...this.setting, ...changed };
		// dom에 반영
		this.updateDom();
		// 저장
		await themeCommands.updateThemeSettings({ patch: changed });
	}

	// =======================================
	// dom에 테마 적용 함수
	// =======================================
	updateDom() {
		this.applyThemeToDom(this.setting);
		this.setupSystemWatcher(this.setting);
		this.applyTheme(this.setting.selectTheme);
	}

	private applyCssVar(isDark: boolean, theme: ThemeSetting) {
		let palette = generateColorPalette(isDark, theme.accentColor);

		// HTML root에 CSS 변수로 등록
		document.documentElement.style.setProperty("font-size", `${theme.fontSize}px`);
		document.body.style.setProperty("--color-text", palette.textColor);
		document.body.style.setProperty("--color-bg-light", palette.bgLightColor);
		document.body.style.setProperty("--accent-color", palette.accentColor);
	}

	// DOM에 테마 적용
	private applyThemeToDom(theme: ThemeSetting) {
		if (typeof window === "undefined") return;

		const isSystemDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
		const isDark = theme.mode === "dark" || (theme.mode === "system" && isSystemDark);

		document.body.classList.toggle("is-frameless", !!theme.isFrameless);
		document.body.classList.toggle("theme-dark", isDark);
		document.body.classList.toggle("theme-light", !isDark);
		this.applyCssVar(isDark, theme);
	}

	// 시스템 테마 변경 감시
	private setupSystemWatcher(theme: ThemeSetting) {
		if (typeof window === "undefined") return;

		if (this.mediaQuery && this.listener) {
			this.mediaQuery.removeEventListener("change", this.listener);
		}

		if (theme.mode === "system") {
			this.mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
			this.listener = () => this.applyThemeToDom(theme);
			this.mediaQuery.addEventListener("change", this.listener);
		}
	}

	// =======================================
	// cssTheme
	// =======================================
	async loadTheme(filename: string) {
		const cssContent = await themeCommands.loadTheme({ filename });
		return cssContent;
	}

	async applyTheme(filename: string) {
		if (filename === "default") {
			if (this.styleTag) {
				this.styleTag.textContent = "";
			}
			return;
		}
		const cssContent = await this.loadTheme(filename);
		this.applyThemeDOM(cssContent);
	}
	applyThemeDOM(cssContent: string) {
		if (!this.styleTag) {
			this.styleTag = document.createElement("style");
			this.styleTag.id = "custom-theme-style";
			this.styleTag.setAttribute("type", "text/css");
			this.styleTag.textContent = cssContent ?? "";
			document.head.appendChild(this.styleTag);
		}

		this.styleTag.textContent = cssContent ?? "";
	}

	async saveTheme(mode: "add" | "edit", filename: string, content: Omit<CssTheme, "filename">) {
		await themeCommands.saveTheme({ filename, content });

		if (mode === "add") {
			this.cssThemes = [...this.cssThemes, { filename: filename, ...content }];
		} else if (mode === "edit") {
			this.cssThemes = this.cssThemes.map((item) =>
				item.filename === filename ? { filename: filename, ...content } : item,
			);
			// 저장 이후 반영 필요
			if (content.cssContent) this.applyThemeDOM(content.cssContent);
		}

	}
	async deleteTheme(filename: string) {
		this.cssThemes = this.cssThemes.filter((theme) => theme.filename !== filename);
		await themeCommands.deleteTheme({ filename });
	}
}

export const themeSetting = new ThemeStore();
