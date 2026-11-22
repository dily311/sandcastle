// 사용자가 고를 수 있는 강조색. 팔레트 밖의 색은 #rrggbb로 직접 고른다 (5-E-14).
// 밝은/어두운 테마에서 대비가 맞게 각각 값을 둔다.
export interface Accent {
	id: string;
	label: string;
	light: string;
	dark: string;
}

export const ACCENTS: Accent[] = [
	{ id: "seaglass", label: "바다 유리", light: "#2f8f8a", dark: "#4fb3ac" },
	{ id: "sunset", label: "노을", light: "#cf6a43", dark: "#e58a63" },
	{ id: "lavender", label: "라벤더", light: "#7262cf", dark: "#9a8cf0" },
	{ id: "rose", label: "장미", light: "#c94f78", dark: "#e57a9c" },
	{ id: "sky", label: "하늘", light: "#3f74c9", dark: "#6d9de6" },
	{ id: "moss", label: "이끼", light: "#5d8a3a", dark: "#86b35e" },
	{ id: "sand", label: "모래", light: "#a87a2f", dark: "#d1a356" },
	{ id: "slate", label: "돌", light: "#58626f", dark: "#9aa6b5" },
];

export const DEFAULT_ACCENT = ACCENTS[0].id;

export const isHexColor = (v: string | null | undefined): v is string => !!v && /^#[0-9a-f]{6}$/i.test(v);

/** 팔레트 id면 화면 모드에 맞는 색, #rrggbb면 그대로 */
export function accentValue(id: string, dark: boolean): string {
	if (isHexColor(id)) return id;
	const a = ACCENTS.find((x) => x.id === id) ?? ACCENTS[0];
	return dark ? a.dark : a.light;
}
