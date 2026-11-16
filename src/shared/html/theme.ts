// iframe(HTML 코드 블록)에 넣을 지금 테마: 앱의 CSS 변수 값을 그대로 옮긴다.
// AI가 쓴 HTML은 var(--accent) 같은 변수를 쓸 수 있고, 아무것도 안 써도 앱과 같은 바탕·글자색으로 보인다.

/** 옮길 변수 (색 · 모서리 · 글꼴) */
export const THEME_VARS = [
	"--bg",
	"--bg-sunken",
	"--surface",
	"--surface-hover",
	"--border",
	"--border-strong",
	"--text",
	"--text-muted",
	"--text-faint",
	"--text-on-accent",
	"--accent",
	"--accent-hover",
	"--accent-soft",
	"--accent-text",
	"--danger",
	"--success",
	"--warning",
	"--info",
	"--radius-sm",
	"--radius-md",
	"--radius-lg",
	"--font-sans",
] as const;

/** 변수 값 → iframe 문서 맨 앞에 넣을 CSS */
export function themeCss(values: Record<string, string>, dark: boolean): string {
	const vars = Object.entries(values)
		.filter(([, v]) => v && !/[<{}]/.test(v))
		.map(([k, v]) => `${k}:${v}`)
		.join(";");
	return (
		`:root{${vars};color-scheme:${dark ? "dark" : "light"}}` +
		`html,body{background:var(--surface);color:var(--text);font-family:var(--font-sans,system-ui,sans-serif)}` +
		`a{color:var(--accent-text)}` +
		`button{font:inherit;color:var(--text-on-accent);background:var(--accent);border:none;border-radius:var(--radius-sm,8px);padding:6px 12px;cursor:pointer}` +
		`button:hover{background:var(--accent-hover,var(--accent))}` +
		`input,textarea,select{font:inherit;color:var(--text);background:var(--bg);border:1px solid var(--border);border-radius:var(--radius-sm,8px);padding:4px 8px}` +
		`hr{border:none;border-top:1px solid var(--border)}`
	);
}

/** 지금 화면의 테마 CSS (계산된 값이라 color-mix 같은 식도 실제 색으로 들어간다) */
export function currentThemeCss(dark: boolean): string {
	if (typeof document === "undefined") return "";
	const cs = getComputedStyle(document.documentElement);
	const values: Record<string, string> = {};
	for (const v of THEME_VARS) values[v] = cs.getPropertyValue(v).trim();
	return themeCss(values, dark);
}
