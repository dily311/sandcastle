// 테마: 화면 모드별로 덮어쓸 CSS 변수 묶음 + 직접 쓰는 CSS.
// - 기본 제공 테마는 여기 들어 있고 (지울 수 없음, 복제해서 고칠 수 있음)
// - 사용자 테마는 앱 데이터 themes 폴더의 JSON (백엔드 ThemeManager)
import { grayScale } from "./grays";
import { accentValue } from "./accents";

export type Mode = "light" | "dark";
export type ColorMap = Record<string, string>;
export interface ThemeColors {
	light: ColorMap;
	dark: ColorMap;
}

/** 색 편집 화면에 보이는 변수 */
export const COLOR_VARS: { group: string; items: { name: string; label: string; hint?: string }[] }[] = [
	{
		group: "바탕",
		items: [
			{ name: "--bg", label: "앱 바탕", hint: "사이드바·헤더·목록" },
			{ name: "--surface-2", label: "본문 바탕", hint: "설정·대화 본문" },
			{ name: "--bg-sunken", label: "묶음 바탕", hint: "설정 카드처럼 한 단계 들어간 곳" },
			{ name: "--surface", label: "카드·입력칸" },
			{ name: "--surface-hover", label: "마우스 올렸을 때" },
			{ name: "--surface-active", label: "선택됨" },
		],
	},
	{
		group: "선",
		items: [
			{ name: "--border", label: "테두리" },
			{ name: "--border-strong", label: "진한 테두리", hint: "입력칸 테두리" },
		],
	},
	{
		group: "글자",
		items: [
			{ name: "--text", label: "본문 글자" },
			{ name: "--text-muted", label: "보조 글자" },
			{ name: "--text-faint", label: "흐린 글자", hint: "시각·번호" },
		],
	},
	{
		group: "강조",
		items: [
			{ name: "--accent", label: "강조색", hint: "비우면 설정에서 고른 강조색" },
			{ name: "--text-on-accent", label: "강조색 위 글자" },
		],
	},
];

export interface BuiltinTheme {
	id: string;
	name: string;
	colors: ThemeColors;
}

const empty: ThemeColors = { light: {}, dark: {} };

export const BUILTIN_THEMES: BuiltinTheme[] = [
	{ id: "default", name: "기본", colors: empty },
	{
		id: "ink",
		name: "잉크",
		colors: {
			light: { "--bg": "#f7f5ee", "--surface-2": "#fbfaf4", "--bg-sunken": "#efece2", "--surface": "#fffdf7", "--surface-hover": "#efece2", "--surface-active": "#e7e3d6", "--border": "#e2ddcf", "--border-strong": "#cfc8b6", "--text": "#1b1c3a", "--text-muted": "#4f5070", "--text-faint": "#8b8ba3" },
			dark: { "--bg": "#14152b", "--surface-2": "#191a33", "--bg-sunken": "#0f1022", "--surface": "#1e1f3a", "--surface-hover": "#262848", "--surface-active": "#2e3054", "--border": "#2c2e4f", "--border-strong": "#3d4068", "--text": "#ecebf7", "--text-muted": "#aeb0cf", "--text-faint": "#71739a" },
		},
	},
	{
		id: "wine",
		name: "와인",
		colors: {
			light: { "--bg": "#fbf3f4", "--surface-2": "#fdf7f8", "--bg-sunken": "#f4e7e9", "--surface": "#ffffff", "--surface-hover": "#f4e7e9", "--surface-active": "#ecd8dc", "--border": "#ead7db", "--border-strong": "#d9bcc2", "--text": "#3d1621", "--text-muted": "#7a4652", "--text-faint": "#ad8590", "--accent": "#a8324f" },
			dark: { "--bg": "#2a181e", "--surface-2": "#311c23", "--bg-sunken": "#211217", "--surface": "#3a222a", "--surface-hover": "#442830", "--surface-active": "#502f39", "--border": "#4a2b34", "--border-strong": "#64404b", "--text": "#f6e7eb", "--text-muted": "#d1aeb8", "--text-faint": "#94707b", "--accent": "#b8e06c", "--text-on-accent": "#24180c" },
		},
	},
	{
		id: "vibrowave",
		name: "바이브로웨이브",
		colors: {
			light: { "--bg": "#f6f2ff", "--surface-2": "#faf7ff", "--bg-sunken": "#ede6fd", "--surface": "#ffffff", "--surface-hover": "#ede6fd", "--surface-active": "#e1d6fb", "--border": "#e1d8f8", "--border-strong": "#c9b9f0", "--text": "#22164f", "--text-muted": "#5a4a91", "--text-faint": "#9586c4", "--accent": "#d6338f" },
			dark: { "--bg": "#100c2a", "--surface-2": "#15103a", "--bg-sunken": "#0b0820", "--surface": "#1b1548", "--surface-hover": "#241c5a", "--surface-active": "#2d236c", "--border": "#2a2160", "--border-strong": "#3f3385", "--text": "#f3ecff", "--text-muted": "#b9acec", "--text-faint": "#7d70b3", "--accent": "#f04aa5" },
		},
	},
	{
		id: "forest",
		name: "숲",
		colors: {
			light: { "--bg": "#f3f6f1", "--surface-2": "#f8faf6", "--bg-sunken": "#e8eee4", "--surface": "#ffffff", "--surface-hover": "#e8eee4", "--surface-active": "#dce5d6", "--border": "#dbe3d5", "--border-strong": "#c1cdb9", "--text": "#16241a", "--text-muted": "#4b5f4f", "--text-faint": "#859686" },
			dark: { "--bg": "#111a14", "--surface-2": "#151f18", "--bg-sunken": "#0c140f", "--surface": "#1a261d", "--surface-hover": "#213025", "--surface-active": "#293a2d", "--border": "#263529", "--border-strong": "#3a4d3d", "--text": "#e7efe8", "--text-muted": "#a8bcaa", "--text-faint": "#6d806f" },
		},
	},
	{
		id: "contrast",
		name: "고대비",
		colors: {
			light: { "--bg": "#ffffff", "--surface-2": "#ffffff", "--bg-sunken": "#f2f2f2", "--surface": "#ffffff", "--surface-hover": "#ebebeb", "--surface-active": "#dedede", "--border": "#8a8a8a", "--border-strong": "#3c3c3c", "--text": "#000000", "--text-muted": "#222222", "--text-faint": "#4a4a4a" },
			dark: { "--bg": "#000000", "--surface-2": "#000000", "--bg-sunken": "#0d0d0d", "--surface": "#0a0a0a", "--surface-hover": "#1f1f1f", "--surface-active": "#2e2e2e", "--border": "#7a7a7a", "--border-strong": "#c4c4c4", "--text": "#ffffff", "--text-muted": "#e6e6e6", "--text-faint": "#b5b5b5" },
		},
	},
];

export const builtinId = (key: string) => (key.startsWith("builtin:") ? key.slice(8) : null);

/** 아무것도 덮어쓰지 않았을 때의 색 (tokens.css와 같은 규칙) */
export function defaultColors(mode: Mode, grayTone: string, accentId: string): ColorMap {
	const g = grayScale(grayTone, mode === "dark");
	const dark = mode === "dark";
	return {
		"--bg": g[0],
		"--surface-2": g[1],
		"--bg-sunken": dark ? mix(g[0], "#000000", 0.3) : g[2],
		"--surface": dark ? g[1] : "#ffffff",
		"--surface-hover": g[2],
		"--surface-active": g[3],
		"--border": g[4],
		"--border-strong": g[6],
		"--text": g[11],
		"--text-muted": g[10],
		"--text-faint": g[8],
		"--accent": accentValue(accentId, dark),
		"--text-on-accent": dark ? "#0d1716" : "#ffffff",
	};
}

/** 테마 색을 CSS로 */
export function colorsToCss(colors: ThemeColors): string {
	const block = (mode: Mode) => {
		const entries = Object.entries(colors[mode] ?? {}).filter(([k, v]) => k.startsWith("--") && v.trim() && k !== "--accent");
		return entries.length ? `:root[data-theme="${mode}"] {\n${entries.map(([k, v]) => `\t${k}: ${v};`).join("\n")}\n}` : "";
	};
	return [block("light"), block("dark")].filter(Boolean).join("\n");
}

// ---------------------------------------------------------
// 예전 테마 JSON (cssContent 안의 body.theme-light / body.theme-dark 블록, 옵시디언식 변수 이름)
// → 새 테마 변수. 테마의 colors가 정한 값이 있으면 그쪽이 이긴다.
// ---------------------------------------------------------

/** 새 변수 ← 예전 변수 (앞에 있는 것부터 찾음) */
const LEGACY_MAP: [string, string[]][] = [
	["--bg", ["--sidebar-bg", "--background-secondary"]],
	["--surface-2", ["--background-primary"]],
	["--bg-sunken", ["--background-secondary", "--track-bg-color"]],
	["--surface", ["--card-bg", "--assistant-bubble-bg", "--background-primary"]],
	["--surface-hover", ["--card-hover-bg", "--nav-bg-hover", "--icon-hover-bg"]],
	["--surface-active", ["--nav-bg-active"]],
	["--border", ["--border-color", "--card-border-color"]],
	["--border-strong", ["--border-strong-color"]],
	["--text", ["--text-color", "--input-color"]],
	["--text-muted", ["--text-muted", "--nav-text"]],
	["--text-faint", ["--text-faint"]],
	["--accent", ["--accent-color", "--text-accent"]],
	["--bubble-ai", ["--assistant-bubble-bg"]],
];

/** css 안의 선택자 블록들에서 변수 선언을 모드별로 모은다 */
function legacyDecls(css: string): Record<Mode, Record<string, string>> {
	const out: Record<Mode, Record<string, string>> = { light: {}, dark: {} };
	const clean = css.replace(/\/\*[\s\S]*?\*\//g, "");
	for (const m of clean.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
		const sel = m[1].trim();
		// body / :root / .theme-light는 밝게(어둡게에도 기본으로), .theme-dark는 어둡게
		const modes: Mode[] = /theme-dark|\[data-theme="?dark/.test(sel) ? ["dark"] : /theme-light|\[data-theme="?light/.test(sel) ? ["light"] : /^(body|:root|html)$/.test(sel) ? ["light", "dark"] : [];
		for (const d of m[2].matchAll(/(--[\w-]+)\s*:\s*([^;]+);?/g)) {
			for (const mode of modes) if (!(mode === "dark" && modes.length === 2 && out.dark[d[1]])) out[mode][d[1]] = d[2].trim();
		}
	}
	return out;
}

/** var(--x) 를 같은 모드 선언으로 풀어 쓴다 (못 풀면 그대로) */
function resolveVar(value: string, decls: Record<string, string>, depth = 0): string {
	if (depth > 8) return value;
	return value.replace(/var\(\s*(--[\w-]+)\s*(?:,\s*([^)]+))?\)/g, (all, name: string, fb?: string) => {
		const v = decls[name] ?? fb;
		return v ? resolveVar(v.trim(), decls, depth + 1) : all;
	});
}

/** 예전 테마 CSS에서 새 테마 색을 뽑는다 (없으면 빈 묶음) */
export function legacyColors(css: string): ThemeColors {
	const decls = legacyDecls(css ?? "");
	const pick = (mode: Mode): ColorMap => {
		const d = mode === "dark" ? decls.dark : decls.light;
		const out: ColorMap = {};
		for (const [to, from] of LEGACY_MAP) {
			const key = from.find((f) => d[f]?.trim());
			if (!key) continue;
			const v = resolveVar(d[key], d);
			if (!/var\(/.test(v)) out[to] = v;
		}
		return out;
	};
	return { light: pick("light"), dark: pick("dark") };
}

/** 테마의 colors 위에 예전 변수에서 뽑은 색을 깐다 (colors가 이김) */
export function withLegacy(colors: ThemeColors, css: string): ThemeColors {
	const legacy = legacyColors(css);
	return { light: { ...legacy.light, ...(colors.light ?? {}) }, dark: { ...legacy.dark, ...(colors.dark ?? {}) } };
}

function mix(a: string, b: string, t: number) {
	const p = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
	const [x, y] = [p(a), p(b)];
	return "#" + x.map((v, i) => Math.round(v + (y[i] - v) * t).toString(16).padStart(2, "0")).join("");
}
