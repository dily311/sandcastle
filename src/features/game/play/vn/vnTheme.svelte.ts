// 비주얼 노벨 / 연애시뮬 플레이 화면 커스텀 테마 설정 (화면 비율 · 프리셋 · 대사창 · 글자 크기 · 강조색 · 스탠딩 크기)

export type VnAspect = "full" | "16:9" | "4:3" | "21:9";
export type VnPreset = "app" | "sakura" | "midnight" | "retro" | "minimal";
export type VnBoxStyle = "card" | "bar" | "classic";

export interface VnThemeConfig {
	aspect: VnAspect;
	preset: VnPreset;
	boxStyle: VnBoxStyle;
	/** 대사창 불투명도 (0.3 ~ 1) */
	dialogueOpacity: number;
	/** 대사 글자 크기 (px, 13 ~ 24) */
	fontSize: number;
	/** 커스텀 강조색 (#rrggbb, null이면 프리셋/앱 기본값) */
	accentColor: string | null;
	/** 캐릭터 스탠딩 크기 배율 (0.7 ~ 1.2) */
	spriteScale: number;
}

export const VN_ASPECTS: { value: VnAspect; label: string }[] = [
	{ value: "full", label: "전체" },
	{ value: "16:9", label: "16:9" },
	{ value: "4:3", label: "4:3" },
	{ value: "21:9", label: "21:9" },
];

export const VN_BOX_STYLES: { value: VnBoxStyle; label: string }[] = [
	{ value: "card", label: "카드" },
	{ value: "bar", label: "하단 바" },
	{ value: "classic", label: "클래식" },
];

export const VN_PRESETS: {
	id: VnPreset;
	name: string;
	bg: string;
	box: string;
	accent: string;
	defaultBoxStyle: VnBoxStyle;
}[] = [
	{ id: "app", name: "앱 기본", bg: "var(--bg-sunken)", box: "var(--surface)", accent: "var(--accent)", defaultBoxStyle: "card" },
	{ id: "sakura", name: "사쿠라 핑크", bg: "#2b1926", box: "#fff4f8", accent: "#ec4899", defaultBoxStyle: "card" },
	{ id: "midnight", name: "미드나잇 노벨", bg: "#0d1024", box: "#171c36", accent: "#818cf8", defaultBoxStyle: "card" },
	{ id: "retro", name: "레트로 90s", bg: "#0a0e1a", box: "#122147", accent: "#38bdf8", defaultBoxStyle: "classic" },
	{ id: "minimal", name: "미니멀 시네마", bg: "#0b0b0e", box: "#141419", accent: "#f59e0b", defaultBoxStyle: "bar" },
];

export const DEFAULT_VN_THEME: VnThemeConfig = {
	aspect: "full",
	preset: "app",
	boxStyle: "card",
	dialogueOpacity: 0.9,
	fontSize: 17,
	accentColor: null,
	spriteScale: 1,
};

const STORAGE_KEY = "sc.vn.theme";

function load(): VnThemeConfig {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return { ...DEFAULT_VN_THEME };
		const p = JSON.parse(raw) as Partial<VnThemeConfig>;
		return {
			aspect: VN_ASPECTS.some((a) => a.value === p.aspect) ? p.aspect! : DEFAULT_VN_THEME.aspect,
			preset: VN_PRESETS.some((x) => x.id === p.preset) ? p.preset! : DEFAULT_VN_THEME.preset,
			boxStyle: VN_BOX_STYLES.some((b) => b.value === p.boxStyle) ? p.boxStyle! : DEFAULT_VN_THEME.boxStyle,
			dialogueOpacity: typeof p.dialogueOpacity === "number" ? Math.min(1, Math.max(0.3, p.dialogueOpacity)) : DEFAULT_VN_THEME.dialogueOpacity,
			fontSize: typeof p.fontSize === "number" ? Math.min(24, Math.max(13, Math.round(p.fontSize))) : DEFAULT_VN_THEME.fontSize,
			accentColor: typeof p.accentColor === "string" && /^#[0-9a-f]{6}$/i.test(p.accentColor) ? p.accentColor : null,
			spriteScale: typeof p.spriteScale === "number" ? Math.min(1.2, Math.max(0.7, p.spriteScale)) : DEFAULT_VN_THEME.spriteScale,
		};
	} catch {
		return { ...DEFAULT_VN_THEME };
	}
}

class VnThemeStore {
	config = $state<VnThemeConfig>(load());

	private save() {
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(this.config));
		} catch {
			/* 무시 */
		}
	}

	update(patch: Partial<VnThemeConfig>) {
		this.config = { ...this.config, ...patch };
		this.save();
	}

	applyPreset(id: VnPreset) {
		const found = VN_PRESETS.find((p) => p.id === id);
		this.update({
			preset: id,
			boxStyle: found?.defaultBoxStyle ?? "card",
		});
	}

	reset() {
		this.config = { ...DEFAULT_VN_THEME };
		this.save();
	}
}

export const vnTheme = new VnThemeStore();
