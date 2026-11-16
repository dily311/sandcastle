// 배경 고르기: 기본 그라데이션 + 올려 둔 이미지 (테마 배경 목록, ThemeSetting.backgrounds)
// 값은 "preset:<id>" 또는 이미지 파일 경로. CSS(테마)와 캔버스(이미지 카드)가 같은 정의를 쓴다.

export interface GradientPreset {
	id: string;
	name: string;
	/** 각도(도)와 색 */
	angle: number;
	stops: string[];
}

export const GRADIENTS: GradientPreset[] = [
	{ id: "dusk", name: "노을", angle: 135, stops: ["#5b4bdb", "#8f6fe0", "#c7a3e8"] },
	{ id: "sky", name: "하늘", angle: 135, stops: ["#8db8f2", "#a9c4ee", "#d8c9e9"] },
	{ id: "aurora", name: "오로라", angle: 135, stops: ["#4f7fdc", "#7aa8d8", "#9fd6c2"] },
	{ id: "pastel", name: "파스텔", angle: 135, stops: ["#a8d8ea", "#c9b6e4", "#f5c6d6"] },
	{ id: "peach", name: "복숭아", angle: 135, stops: ["#ffd3a5", "#fd9c8b", "#f38fb1"] },
	{ id: "mint", name: "민트", angle: 135, stops: ["#d4fc79", "#96e6a1", "#84d3c4"] },
	{ id: "ocean", name: "바다", angle: 160, stops: ["#0f2b4c", "#1f5f8b", "#5aa9c7"] },
	{ id: "night", name: "밤", angle: 160, stops: ["#0b0f1f", "#1b2240", "#3a2f5b"] },
	{ id: "rose", name: "장미", angle: 135, stops: ["#5a1832", "#a83b5c", "#e8a0a8"] },
	{ id: "forest", name: "숲", angle: 160, stops: ["#13241a", "#2f5a3a", "#8cbf88"] },
	{ id: "sand", name: "모래", angle: 135, stops: ["#efe6d6", "#e2cfae", "#c9a87c"] },
	{ id: "mono", name: "먹", angle: 160, stops: ["#1a1a1a", "#3a3a3a", "#6b6b6b"] },
];

export const presetId = (value: string | null | undefined) => (value?.startsWith("preset:") ? value.slice(7) : null);
export const gradientOf = (value: string | null | undefined) => GRADIENTS.find((g) => g.id === presetId(value)) ?? null;

export function gradientCss(g: GradientPreset): string {
	return `linear-gradient(${g.angle}deg, ${g.stops.join(", ")})`;
}

/** 캔버스에 그라데이션 칠하기 (CSS 각도와 같은 방향) */
export function fillGradient(ctx: CanvasRenderingContext2D, g: GradientPreset, w: number, h: number) {
	const rad = ((g.angle - 90) * Math.PI) / 180;
	const half = (Math.abs(w * Math.cos(rad)) + Math.abs(h * Math.sin(rad))) / 2;
	const cx = w / 2;
	const cy = h / 2;
	const grad = ctx.createLinearGradient(cx - Math.cos(rad) * half, cy - Math.sin(rad) * half, cx + Math.cos(rad) * half, cy + Math.sin(rad) * half);
	g.stops.forEach((c, i) => grad.addColorStop(g.stops.length === 1 ? 0 : i / (g.stops.length - 1), c));
	ctx.fillStyle = grad;
	ctx.fillRect(0, 0, w, h);
}

/** 이미지를 가득 채워 그리기 (object-fit: cover) */
export function drawCover(ctx: CanvasRenderingContext2D, img: CanvasImageSource & { width: number; height: number }, w: number, h: number) {
	const s = Math.max(w / img.width, h / img.height);
	const dw = img.width * s;
	const dh = img.height * s;
	ctx.drawImage(img, (w - dw) / 2, (h - dh) / 2, dw, dh);
}
