// 테마: 화면 모드(시스템/밝게/어둡게), 고정 강조색, 회색 톤, 글자 크기, 테마(색 + CSS), CSS 스니펫
import { jobs } from "$shared/stores/jobs.svelte";
import type { CssSnippet, ThemeChatSetting, ThemeSetting } from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ACCENTS, DEFAULT_ACCENT, accentValue, isHexColor } from "$shared/ui/accents";
import { DEFAULT_GRAY, grayScale } from "$shared/ui/grays";
import { BUILTIN_THEMES, builtinId, colorsToCss, withLegacy, type ThemeColors } from "$shared/ui/themes";
import { fileSrc } from "$shared/ipc/file";
import { onEvent } from "$shared/ipc/events";
import { gradientCss, gradientOf } from "$shared/ui/backgrounds";
import { applyUserFont } from "$shared/ui/userFont";

export type { CssSnippet, ThemeChatSetting };

const DEFAULT_CHAT: ThemeChatSetting = {
	showTimestamps: true,
	showAvatars: true,
	showMultiCharAvatars: false,
	charAvatarFrame: "circle",
	charAvatarFrameImage: null,
	userAvatarFrame: "circle",
	userAvatarFrameImage: null,
};

const FALLBACK: ThemeSetting = {
	mode: "system",
	selectTheme: "default",
	isFrameless: true,
	accentColor: DEFAULT_ACCENT,
	fontSize: 16,
	grayTone: DEFAULT_GRAY,
	snippets: [],
	backgroundImage: null,
	backgrounds: [],
	backgroundOpacity: 0.82,
	fonts: [],
	appFont: null,
	textFont: null,
	monoFont: null,
	chat: DEFAULT_CHAT,
};

/** 지금 적용할 테마 내용 */
interface Active {
	colors: ThemeColors;
	css: string;
	/** 테마가 정한 강조색·회색 톤·대화 표시 설정 (없으면 설정) */
	accent?: string | null;
	gray?: string | null;
	chat?: ThemeChatSetting | null;
}
const NONE: Active = { colors: { light: {}, dark: {} }, css: "" };

class Theme {
	setting = $state<ThemeSetting>(FALLBACK);
	systemDark = $state(false);
	/** 고른 테마의 색·CSS·대화 표시 설정 (불러온 뒤) */
	private active = $state<Active>(NONE);
	/** 테마 편집 중 미리보기 (있으면 고른 테마 대신 씀) */
	private previewing: Active | null = null;

	get dark() {
		return this.setting.mode === "dark" || (this.setting.mode === "system" && this.systemDark);
	}
	/** 강조색: 팔레트 id 또는 직접 고른 #rrggbb. 모르는 값이면 기본색 */
	get accentId() {
		const a = this.setting.accentColor;
		return ACCENTS.some((x) => x.id === a) || isHexColor(a) ? a : DEFAULT_ACCENT;
	}
	/** 테마가 정했으면 그것, 아니면 설정 */
	private get effAccent() {
		const a = this.active.accent;
		return a && (ACCENTS.some((x) => x.id === a) || isHexColor(a)) ? a : this.accentId;
	}
	private get effGray() {
		return this.active.gray || this.setting.grayTone || DEFAULT_GRAY;
	}
	get snippets(): CssSnippet[] {
		return this.setting.snippets ?? [];
	}
	/** 대화 화면 표시 설정: 고른 커스텀 테마가 정했으면 그것, 아니면 테마 설정 */
	get chat(): ThemeChatSetting {
		const c = this.active.chat ?? this.setting.chat;
		return {
			showTimestamps: c?.showTimestamps ?? DEFAULT_CHAT.showTimestamps,
			showAvatars: c?.showAvatars ?? DEFAULT_CHAT.showAvatars,
			showMultiCharAvatars: c?.showMultiCharAvatars ?? DEFAULT_CHAT.showMultiCharAvatars,
			charAvatarFrame: c?.charAvatarFrame ?? DEFAULT_CHAT.charAvatarFrame,
			charAvatarFrameImage: c?.charAvatarFrameImage ?? DEFAULT_CHAT.charAvatarFrameImage,
			userAvatarFrame: c?.userAvatarFrame ?? DEFAULT_CHAT.userAvatarFrame,
			userAvatarFrameImage: c?.userAvatarFrameImage ?? DEFAULT_CHAT.userAvatarFrameImage,
		};
	}
	async updateChat(patch: Partial<ThemeChatSetting>) {
		const cur = this.setting.chat ?? DEFAULT_CHAT;
		await this.update({ chat: { ...DEFAULT_CHAT, ...cur, ...patch } });
	}
	async uploadAvatarFrame(target: "char" | "user", path: string) {
		this.setting = await call(commands.uploadAvatarFrame(target, path), { source: "theme" });
		this.apply();
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
		// 다른 곳(갤러리에서 배경 그림 삭제, 위젯 창 등)에서 바뀐 설정을 따라간다
		void onEvent<ThemeSetting>("theme_setting::changed", async (next) => {
			const themeChanged = next.selectTheme !== this.setting.selectTheme;
			this.setting = next;
			if (themeChanged) await this.loadActive();
			this.apply();
		});
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
			this.active = t
				? {
						colors: withLegacy({ light: t.colors.light ?? {}, dark: t.colors.dark ?? {} }, t.cssContent ?? ""),
						css: t.cssContent ?? "",
						accent: t.accentColor ?? null,
						gray: t.grayTone ?? null,
						chat: t.chat ?? null,
					}
				: NONE;
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
		root.style.setProperty("--accent", cur.colors[mode]["--accent"]?.trim() || accentValue(this.effAccent, this.dark));
		grayScale(this.effGray, this.dark).forEach((c, i) => root.style.setProperty(`--gray-${i + 1}`, c));
		root.style.fontSize = `${this.setting.fontSize}px`;
		applyUserFont(root, this.setting);
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
		const value = this.setting.backgroundImage;
		const gradient = gradientOf(value);
		const url = gradient ? null : value ? fileSrc(value) : null;
		if (!url && !gradient) {
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
			gradient
				? `:root[data-bg-image] body { background: ${gradientCss(gradient)} fixed; }`
				: `:root[data-bg-image] body { background: #000 url("${(url ?? "").replace(/"/g, "%22")}") center / cover no-repeat fixed; }`,
		].join("\n");
	}

	/** 앱 배경 고르기: 배경 목록의 경로 / "preset:<id>" / null(없음) */
	async setBackground(value: string | null) {
		await this.update({ backgroundImage: value });
	}
	/** 배경 목록에 이미지 올리기 (select면 마지막 것을 앱 배경으로) */
	async addBackgrounds(paths: string[], select = false) {
		if (!paths.length) return;
		const s = await jobs.run(`배경 ${paths.length}장`, () => call(commands.addBackgrounds(paths, select), { source: "theme" }), { group: "배경 올리기" });
		if (!s) return;
		this.setting = s;
		this.apply();
	}
	/** 글꼴 올리기: 앱 데이터에 복사. select면 마지막 것을 앱 글꼴로 */
	async addFonts(paths: string[], select = false) {
		if (!paths.length) return;
		const s = await jobs.run(`글꼴 ${paths.length}개`, () => call(commands.addFonts(paths, select), { source: "theme" }), { group: "글꼴 올리기" });
		if (!s) return;
		this.setting = s;
		this.apply();
	}
	async removeFont(path: string) {
		this.setting = await call(commands.removeFont(path), { source: "theme" });
		this.apply();
	}
	async removeBackground(path: string) {
		this.setting = await call(commands.removeBackground(path), { source: "theme" });
		this.apply();
	}

	/** 프레임 없는 창이면 앱 헤더가 창 버튼을 그림, 아니면 운영체제 제목 표시줄 */
	private applyFrame() {
		// 창 테두리는 데스크톱만 (Android에는 창이 하나뿐)
		if (!__IS_TAURI__ || /Android|iPhone|iPad|iPod/i.test(navigator.userAgent)) return;
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
