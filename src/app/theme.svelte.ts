// 테마: 화면 모드(시스템/밝게/어둡게), 고정 강조색, 회색 톤, 글자 크기, 테마(색 + CSS), CSS 스니펫
import type { CssSnippet, ThemeSetting } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ACCENTS, DEFAULT_ACCENT, accentValue } from "$shared/ui/accents";
import { DEFAULT_GRAY, grayScale } from "$shared/ui/grays";
import { BUILTIN_THEMES, builtinId, colorsToCss, withLegacy, type ThemeColors } from "$shared/ui/themes";
import { fileSrc } from "$shared/ipc/file";

export type { CssSnippet };

const FALLBACK: ThemeSetting = {
	mode: "system",
	selectTheme: "default",
	isFrameless: true,
	accentColor: DEFAULT_ACCENT,
	fontSize: 16,
	grayTone: DEFAULT_GRAY,
	snippets: [],
	backgroundImage: null,
	backgroundOpacity: 0.82,
};

/** 지금 적용할 테마 내용 */
interface Active {
	colors: ThemeColors;
	css: string;
}
const NONE: Active = { colors: { light: {}, dark: {} }, css: "" };

class Theme {
	setting = $state<ThemeSetting>(FALLBACK);
	systemDark = $state(false);
	/** 고른 테마의 색·CSS (불러온 뒤) */
	private active: Active = NONE;
	/** 테마 편집 중 미리보기 (있으면 고른 테마 대신 씀) */
	private previewing: Active | null = null;

	get dark() {
		return this.setting.mode === "dark" || (this.setting.mode === "system" && this.systemDark);
	}
	/** 예전 버전은 accentColor에 "#rrggbb"를 저장했다 → 고정 팔레트에 없으면 기본색 */
	get accentId() {
		return ACCENTS.some((a) => a.id === this.setting.accentColor) ? this.setting.accentColor : DEFAULT_ACCENT;
	}
	get snippets(): CssSnippet[] {
		return this.setting.snippets ?? [];
	}

	async init() {
		const mq = window.matchMedia("(prefers-color-scheme: dark)");
		this.systemDark = mq.matches;
		mq.addEventListener("change", (e) => {
			this.systemDark = e.matches;
			this.apply();
		});
		this.setting = await call(commands.getThemeSetting(), { source: "theme", silent: true }).catch(() => FALLBACK);
		await this.loadActive();
		this.apply();
		this.applyFrame();
	}

	async update(patch: Partial<ThemeSetting>) {
		this.setting = { ...this.setting, ...patch };
		if ("selectTheme" in patch) await this.loadActive();
		this.apply();
		if ("isFrameless" in patch) this.applyFrame();
		await call(commands.updateThemeSetting(patch), { source: "theme" });
	}

	/** 고른 테마 다시 읽기 (편집 저장 뒤) */
	async reload() {
		await this.loadActive();
		this.apply();
	}

	private async loadActive() {
		const key = this.setting.selectTheme || "default";
		const bid = builtinId(key);
		if (key === "default") this.active = NONE;
		else if (bid) this.active = { colors: BUILTIN_THEMES.find((t) => t.id === bid)?.colors ?? NONE.colors, css: "" };
		else {
			const t = await call(commands.loadTheme(key), { source: "theme", silent: true }).catch(() => null);
			// 예전 테마 JSON(옵시디언식 변수)도 새 변수로 이어서 보이게
			this.active = t ? { colors: withLegacy({ light: t.colors.light ?? {}, dark: t.colors.dark ?? {} }, t.cssContent ?? ""), css: t.cssContent ?? "" } : NONE;
		}
	}

	/** 테마 편집 화면이 고치는 동안 바로 보여줌. null이면 끝 */
	preview(draft: Active | null) {
		this.previewing = draft;
		this.apply();
	}

	apply() {
		const root = document.documentElement;
		const mode = this.dark ? "dark" : "light";
		const cur = this.previewing ?? this.active;
		root.dataset.theme = mode;
		// 테마가 강조색을 정했으면 그것, 아니면 설정의 강조색
		root.style.setProperty("--accent", cur.colors[mode]["--accent"]?.trim() || accentValue(this.accentId, this.dark));
		grayScale(this.setting.grayTone ?? DEFAULT_GRAY, this.dark).forEach((c, i) => root.style.setProperty(`--gray-${i + 1}`, c));
		root.style.fontSize = `${this.setting.fontSize}px`;
		this.style("user-theme").textContent = [colorsToCss(cur.colors), cur.css].filter((s) => s.trim()).join("\n\n");
		this.applyBackground();
		this.style("user-snippets").textContent = this.snippets
			.filter((s) => s.enabled && s.css.trim())
			.map((s) => `/* ${s.name.replace(/\*\//g, "")} */\n${s.css}`)
			.join("\n\n");
	}

	/** 배경 이미지: body에 깔고, 바탕 색들을 불투명도만큼 비치게 한다 */
	private applyBackground() {
		const root = document.documentElement;
		const el = this.style("user-background");
		const url = this.setting.backgroundImage ? fileSrc(this.setting.backgroundImage) : null;
		if (!url) {
			delete root.dataset.bgImage;
			el.textContent = "";
			return;
		}
		// 지금 테마의 바탕 색을 읽어 반투명으로 (배경 규칙을 빼고 계산해야 원래 색이 나온다)
		el.textContent = "";
		const cs = getComputedStyle(root);
		const a = Math.round(Math.min(1, Math.max(0.3, this.setting.backgroundOpacity ?? 0.82)) * 100);
		const see = (name: string) => `${name}: color-mix(in srgb, ${cs.getPropertyValue(name).trim() || "transparent"} ${a}%, transparent);`;
		root.dataset.bgImage = "";
		el.textContent = [
			`:root[data-bg-image] { ${["--bg", "--surface-2", "--bg-sunken"].map(see).join(" ")} }`,
			`:root[data-bg-image] body { background: var(--bg-solid, #000) url("${url.replace(/"/g, "%22")}") center / cover no-repeat fixed; }`,
		].join("\n");
	}

	/** 배경 이미지 고르기 (null이면 없앰) */
	async setBackground(path: string | null) {
		this.setting = await call(commands.setThemeBackground(path), { source: "theme" });
		this.apply();
	}

	/** 프레임 없는 창이면 앱 헤더가 창 버튼을 그림, 아니면 운영체제 제목 표시줄 */
	private applyFrame() {
		if (!__IS_TAURI__) return;
		getCurrentWindow()
			.setDecorations(!this.setting.isFrameless)
			.catch(() => {});
	}

	/** 순서: 테마 → 스니펫 (스니펫이 테마를 덮어씀) */
	private style(id: string) {
		let el = document.getElementById(id) as HTMLStyleElement | null;
		if (!el) {
			el = document.createElement("style");
			el.id = id;
			document.head.appendChild(el);
			if (id === "user-theme") document.getElementById("user-snippets")?.before(el);
			if (id === "user-background") document.getElementById("user-snippets")?.before(el);
		}
		return el;
	}

	// ---- 스니펫 ----
	async saveSnippets(list: CssSnippet[]) {
		await this.update({ snippets: list });
	}
}

export const theme = new Theme();
