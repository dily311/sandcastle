// 고른 글을 이미지 카드(png)로. 캔버스에 직접 그린다 (외부 라이브러리 없음).
// 배경(그라데이션·올린 이미지)을 깔면 글은 반투명 판 위에 올려 읽히게 한다.
import { drawCover, fillGradient, gradientOf } from "$shared/ui/backgrounds";
import { fileSrc } from "$shared/ipc/file";

export type CardStyle = "theme" | "paper" | "night" | "light" | "dark";
export type CardFont = "app" | "sans" | "serif" | "mono";

export const CARD_STYLES: { value: CardStyle; label: string }[] = [
	{ value: "theme", label: "지금 테마" },
	{ value: "paper", label: "종이" },
	{ value: "night", label: "밤" },
	{ value: "light", label: "밝게" },
	{ value: "dark", label: "어둡게" },
];
export const CARD_FONTS: { value: CardFont; label: string }[] = [
	{ value: "app", label: "앱 글꼴" },
	{ value: "sans", label: "고딕" },
	{ value: "serif", label: "명조" },
	{ value: "mono", label: "고정폭" },
];
/** 0 = 글 길이에 맞춰 자동 */
export const CARD_SIZES: { value: string; label: string }[] = [
	{ value: "0", label: "자동" },
	{ value: "32", label: "작게" },
	{ value: "40", label: "보통" },
	{ value: "48", label: "크게" },
	{ value: "60", label: "아주 크게" },
];

export interface CardInput {
	text: string;
	/** 말한 사람 (넣지 않으면 없음) */
	speaker?: string;
	/** 아래 작은 글씨 (답변 시간 등) */
	caption?: string;
	style: CardStyle;
	font: CardFont;
	/** 글자 크기(px), 0이면 자동 */
	size: number;
	/** 배경: null / "preset:<id>" / 이미지 경로 */
	background: string | null;
}

const W = 1080;
const PAD = 96;

function cssVar(name: string, fallback: string) {
	const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	return v || fallback;
}

function palette(style: CardStyle) {
	switch (style) {
		case "paper":
			return { bg: "#f6f1e7", text: "#2b2620", muted: "#8a7f70", accent: "#b0643a" };
		case "night":
			return { bg: "#14161c", text: "#e9eaf0", muted: "#8b90a0", accent: "#8ab4ff" };
		case "light":
			return { bg: "#ffffff", text: "#1a1a1a", muted: "#6b6b6b", accent: cssVar("--accent", "#5b6cff") };
		case "dark":
			return { bg: "#101010", text: "#f2f2f2", muted: "#9a9a9a", accent: cssVar("--accent", "#8ab4ff") };
		default:
			return { bg: cssVar("--surface", "#ffffff"), text: cssVar("--text", "#1a1a1a"), muted: cssVar("--text-muted", "#666666"), accent: cssVar("--accent", "#5b6cff") };
	}
}

function fontFamily(font: CardFont) {
	switch (font) {
		case "sans":
			return `"Pretendard", "Noto Sans KR", "Malgun Gothic", system-ui, sans-serif`;
		case "serif":
			return `"Noto Serif KR", "Nanum Myeongjo", "Batang", Georgia, serif`;
		case "mono":
			return `"D2Coding", "Consolas", ui-monospace, monospace`;
		default:
			return cssVar("--font-sans", "system-ui, sans-serif");
	}
}

/** 폭에 맞춰 줄을 나눈다. 띄어쓰기가 있으면 낱말 단위, 너무 긴 낱말(한 덩어리 글)은 글자 단위 */
export function wrapLines(ctx: Pick<CanvasRenderingContext2D, "measureText">, text: string, maxWidth: number): string[] {
	const out: string[] = [];
	for (const para of text.replace(/\r/g, "").split("\n")) {
		if (!para.trim()) {
			out.push("");
			continue;
		}
		let line = "";
		for (const word of para.split(/(\s+)/)) {
			if (!word) continue;
			const tryLine = line + word;
			if (ctx.measureText(tryLine).width <= maxWidth) {
				line = tryLine;
				continue;
			}
			if (line.trim()) out.push(line.trimEnd());
			line = word.trimStart();
			// 낱말 하나가 폭보다 길면 글자로 자른다
			while (ctx.measureText(line).width > maxWidth) {
				let cut = line.length - 1;
				while (cut > 1 && ctx.measureText(line.slice(0, cut)).width > maxWidth) cut--;
				out.push(line.slice(0, cut));
				line = line.slice(cut);
			}
		}
		if (line.trim()) out.push(line.trimEnd());
	}
	while (out.length && !out[out.length - 1]) out.pop();
	return out;
}

const imageCache = new Map<string, Promise<HTMLImageElement | null>>();
function loadImage(path: string): Promise<HTMLImageElement | null> {
	let p = imageCache.get(path);
	if (!p) {
		p = new Promise((resolve) => {
			const src = fileSrc(path);
			if (!src) return resolve(null);
			const img = new Image();
			img.crossOrigin = "anonymous";
			img.onload = () => resolve(img);
			img.onerror = () => resolve(null);
			img.src = src;
		});
		imageCache.set(path, p);
	}
	return p;
}

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
	ctx.beginPath();
	ctx.moveTo(x + r, y);
	ctx.arcTo(x + w, y, x + w, y + h, r);
	ctx.arcTo(x + w, y + h, x, y + h, r);
	ctx.arcTo(x, y + h, x, y, r);
	ctx.arcTo(x, y, x + w, y, r);
	ctx.closePath();
}

/** 카드를 그린 캔버스 */
export async function renderCard(input: CardInput): Promise<HTMLCanvasElement> {
	const c = palette(input.style);
	const family = fontFamily(input.font);
	const text = input.text.trim().slice(0, 2000);
	const size = input.size > 0 ? input.size : text.length > 600 ? 34 : text.length > 250 ? 40 : 48;
	const lh = Math.round(size * 1.6);
	const gradient = gradientOf(input.background);
	const bgImage = !gradient && input.background ? await loadImage(input.background) : null;
	const framed = !!(gradient || bgImage);
	// 배경이 있으면 글 판을 안쪽으로 들여 그린다
	const inset = framed ? 72 : 0;
	const pad = PAD - (framed ? 24 : 0);

	const measure = document.createElement("canvas").getContext("2d")!;
	measure.font = `500 ${size}px ${family}`;
	const lines = wrapLines(measure, text, W - inset * 2 - pad * 2);

	const head = input.speaker ? Math.round(size * 1.5) : 0;
	const foot = input.caption ? 80 : 24;
	const panelH = pad * 2 + 30 + head + lines.length * lh + foot;
	const H = Math.max(Math.round(W * 0.6), panelH + inset * 2);

	const canvas = document.createElement("canvas");
	canvas.width = W;
	canvas.height = H;
	const ctx = canvas.getContext("2d")!;

	// 바탕
	if (gradient) fillGradient(ctx, gradient, W, H);
	else if (bgImage) drawCover(ctx, bgImage, W, H);
	else {
		ctx.fillStyle = c.bg;
		ctx.fillRect(0, 0, W, H);
	}
	const px = inset;
	const py = framed ? Math.round((H - panelH) / 2) : 0;
	const pw = W - inset * 2;
	const ph = framed ? panelH : H;
	if (framed) {
		ctx.save();
		ctx.shadowColor = "rgba(0,0,0,0.18)";
		ctx.shadowBlur = 40;
		ctx.globalAlpha = 0.88;
		ctx.fillStyle = c.bg;
		roundRect(ctx, px, py, pw, ph, 36);
		ctx.fill();
		ctx.restore();
	}

	// 왼쪽 강조 막대와 큰 따옴표
	ctx.fillStyle = c.accent;
	ctx.fillRect(px + pad - 40, py + pad, 8, ph - pad * 2);
	ctx.globalAlpha = 0.18;
	ctx.font = `700 220px Georgia, serif`;
	ctx.fillText("“", px + pw - pad - 120, py + pad + 150);
	ctx.globalAlpha = 1;

	let y = py + pad + 30;
	if (input.speaker) {
		ctx.fillStyle = c.accent;
		ctx.font = `700 ${Math.round(size * 0.7)}px ${family}`;
		ctx.fillText(input.speaker, px + pad, y + size * 0.7);
		y += head;
	}
	ctx.fillStyle = c.text;
	ctx.font = `500 ${size}px ${family}`;
	for (const line of lines) {
		y += lh;
		ctx.fillText(line, px + pad, y - (lh - size) / 2);
	}
	if (input.caption) {
		ctx.fillStyle = c.muted;
		ctx.font = `400 28px ${family}`;
		ctx.fillText(input.caption, px + pad, py + ph - pad + 10);
	}
	return canvas;
}

export async function cardBlob(canvas: HTMLCanvasElement): Promise<Blob> {
	return new Promise((resolve, reject) => canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("이미지를 만들 수 없음"))), "image/png"));
}

export async function blobBase64(blob: Blob): Promise<string> {
	const buf = new Uint8Array(await blob.arrayBuffer());
	let s = "";
	for (let i = 0; i < buf.length; i += 0x8000) s += String.fromCharCode(...buf.subarray(i, i + 0x8000));
	return btoa(s);
}
