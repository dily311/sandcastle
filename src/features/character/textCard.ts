// 고른 글을 이미지 카드(png)로. 캔버스에 직접 그린다 (외부 라이브러리 없음).
// 디자인(각주·말풍선·편지·자막·미니멀) × 색 묶음 × 사용자가 고친 색 × 배경(그라데이션·이미지)
import { drawCover, fillGradient, gradientOf } from "$shared/ui/backgrounds";
import { fileSrc } from "$shared/ipc/file";

export type CardImgRatio = "1:1" | "3:4" | "5:4" | "9:16";
export type CardStyle = "theme" | "paper" | "night" | "light" | "dark";
export type CardFont = "app" | "sans" | "serif" | "mono";
/** 글 정렬. auto = 디자인 기본값 (미니멀·자막은 가운데, 나머지는 왼쪽) */
export type CardAlign = "start" | "left" | "center" | "right";
export type CardDesign = "none" | "footnote" | "bubble" | "letter" | "subtitle" | "minimal";

export const CARD_IMG_RATIO: { value: CardImgRatio; label: string }[] = [
	{ value: "1:1", label: "1:1"},
	{ value: "3:4", label: "3:4"},
	{ value: "5:4", label: "5:4"},
	{ value: "9:16", label: "9:16"},
];

export const CARD_DESIGNS: { value: CardDesign; label: string }[] = [
	{ value: "none", label: "없음" },
	{ value: "footnote", label: "각주" },
	{ value: "bubble", label: "말풍선" },
	{ value: "letter", label: "편지" },
	{ value: "subtitle", label: "영화 자막" },
	{ value: "minimal", label: "미니멀" },
];
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

export interface CardColors {
	bg: string;
	text: string;
	muted: string;
	accent: string;
}

export interface CardInput {
	text: string;
	/** 말한 사람 (넣지 않으면 없음) */
	speaker?: string;
	/** 작은 글씨 (답변 시간 등) */
	caption?: string;
	/** 이미지 비율 */
	imgRatio: CardImgRatio;
	/** 디자인 */
	design: CardDesign;
	style: CardStyle;
	/** 사용자가 고친 색 (빈 값은 색 묶음 그대로) */
	colors?: Partial<CardColors>;
	txtShadow?: boolean;
	font: CardFont;
	bold: number;
	/** 글 정렬 (없으면 디자인 기본) */
	align: CardAlign;
	/** 글자 크기(px), 0이면 자동 */
	size: number;
	/** 배경: null / "preset:<id>" / 이미지 경로 */
	background: string | null;
}

const W = 1080;

function cssVar(name: string, fallback: string) {
	const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	return v || fallback;
}

/** 색 묶음의 기본 색 (색 고르기 칸의 시작 값) */
export function palette(style: CardStyle): CardColors {
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

function withAlpha(hex: string, a: number) {
	const m = /^#([0-9a-f]{6})$/i.exec(hex.trim());
	if (!m) return hex;
	const n = parseInt(m[1], 16);
	return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
}

/** 카드를 그린 캔버스 */
export async function renderCard(input: CardInput): Promise<HTMLCanvasElement> {
	const c: CardColors = { ...palette(input.style), ...Object.fromEntries(Object.entries(input.colors ?? {}).filter(([, v]) => v)) };
	const family = fontFamily(input.font);
	const bold = input.bold;
	const text = input.text.trim().slice(0, 2000);
	const design = input.design;
	const auto = text.length > 600 ? 34 : text.length > 250 ? 40 : design === "minimal" ? 54 : 48;
	const gradient = gradientOf(input.background);
	const bgImage = !gradient && input.background ? await loadImage(input.background) : null;
	const hasBg = !!(gradient || bgImage);

	// 디자인별 판(글이 놓이는 곳) 여백
	const inset = design === "subtitle" || design === "none" ? 0 : hasBg || design === "bubble" || design === "letter" ? 72 : 0;
	const pad = design === "subtitle" ? 80 : design === "minimal" || design === "none" ? 110 : 84;
	const textW = W - inset * 2 - pad * 2;
	// 정렬: 직접 고르지 않으면 미니멀·자막만 가운데 ("없음"은 왼쪽)
	const align: CanvasTextAlign = input.align ?? "start";
	/** 글을 놓을 x: 왼쪽·가운데·오른쪽 기준점 (양옆 여백 l, r) */
	const anchor = (l: number, r: number) => (align === "center" ? (l + r) / 2 : align === "right" ? r : l);
	const captionH = input.caption ? 70 : 0;
	const tailH = design === "bubble" ? 60 : 0;
	const measure = document.createElement("canvas").getContext("2d")!;

	// 3:4 세로 카드로 고정: 글이 넘치면 글자를 줄여 맞춘다
	let H = W;
	switch (input.imgRatio) {
		default:
		case "1:1":
			break;
		case "3:4":
			H = Math.round((W * 4) / 3);
			break;
		case "5:4":
			H = Math.round((W * 4) / 5);
			break;
		case "9:16":
			H = Math.round((W * 16) / 9);
			break;
	}

	let size = input.size > 0 ? input.size : auto;
	let lh = 0;
	let lines: string[] = [];
	let speakerH = 0;
	let panelH = 0;
	for (;;) {
		lh = Math.round(size * (design === "letter" ? 1.9 : 1.6));
		measure.font = `${bold} ${size}px ${family}`;
		lines = wrapLines(measure, text, textW);
		speakerH = input.speaker ? Math.round(size * 1.5) : 0;
		panelH = pad * 2 + lines.length * lh + speakerH + captionH + (design === "footnote" ? 30 : 0);
		const need = panelH + inset * 2 + tailH + (design === "bubble" && input.speaker ? speakerH : 0);
		if (need <= H || size <= 18) break;
		size -= 2;
	}
	const bodyH = lines.length * lh;

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

	// 텍스트 설정
	ctx.textAlign = align;
	ctx.lineWidth = 1;
	
	if (input.txtShadow) {
		ctx.shadowColor = "rgba(0,0,0,0.5)";
		ctx.shadowBlur = 6;
	}
	
	// 디자인별 추가 설정
	if (design === "none") {
		// 장식 없음: 배경을 살리고 글만 (배경이 있으면 흰 글씨 + 그림자). 정렬은 왼쪽이 기본
	
		const nx = anchor(pad, W - pad);
		let y = Math.round((H - (bodyH + speakerH + captionH)) / 2);
		const fg = input.colors?.text || (hasBg ? "#ffffff" : c.text);
		
		if (input.speaker) {
			ctx.fillStyle = input.colors?.accent || (hasBg ? "#ffffff" : c.accent);
			ctx.font = `700 ${Math.round(size * 0.66)}px ${family}`;
			ctx.fillText(input.speaker, nx, y + size * 0.66);
			y += speakerH;
		}
		ctx.fillStyle = fg;
		ctx.font = `${bold} ${size}px ${family}`;
		for (const line of lines) {
			y += lh;
			ctx.fillText(line, nx, y - (lh - size) / 2);
		}
		if (input.caption) {
			ctx.font = `400 28px ${family}`;
			ctx.fillStyle = hasBg ? "rgba(255,255,255,0.8)" : c.muted;
			ctx.fillText(input.caption, nx, y + 56);
		}
		ctx.shadowBlur = 0;
		return canvas;
	}

	if (design === "subtitle") {
		// 아래쪽 어두운 띠 위에 흰 글씨 (가운데)
		const bandH = Math.min(H, bodyH + speakerH + captionH + pad * 2);
		const grad = ctx.createLinearGradient(0, H - bandH - 80, 0, H);
		grad.addColorStop(0, "rgba(0,0,0,0)");
		grad.addColorStop(0.35, "rgba(0,0,0,0.55)");
		grad.addColorStop(1, "rgba(0,0,0,0.8)");
		ctx.fillStyle = grad;
		ctx.fillRect(0, H - bandH - 80, W, bandH + 80);
		let y = H - bandH + pad * 0.6;
		const sx = anchor(pad, W - pad);
		if (input.speaker) {
			ctx.fillStyle = input.colors?.accent || "#eee";
			ctx.font = `700 ${Math.round(size * 0.62)}px ${family}`;
			ctx.fillText(input.speaker, sx, y + size * 0.6);
			y += speakerH;
		}
		ctx.font = `${bold} ${size}px ${family}`;
		ctx.shadowColor = "rgba(0,0,0,0.8)";
		ctx.shadowBlur = 8;
		ctx.fillStyle = input.colors?.text || "#ffffff";
		for (const line of lines) {
			y += lh;
			ctx.fillText(line, sx, y - (lh - size) / 2);
		}
		ctx.shadowBlur = 0;
		if (input.caption) {
			ctx.fillStyle = "rgba(255,255,255,0.7)";
			ctx.font = `400 26px ${family}`;
			ctx.fillText(input.caption, sx, y + 50);
		}
		return canvas;
	}

	// 판 위치
	const px = inset;
	const pw = W - inset * 2;
	let py = Math.round((H - panelH - tailH - (design === "bubble" ? speakerH : 0)) / 2);
	if (design === "bubble" && input.speaker) {
		// 말한 사람은 말풍선 위 바깥에
		ctx.textAlign = "left";
		ctx.fillStyle = hasBg ? "#ffffff" : c.accent;
		ctx.font = `700 ${Math.round(size * 0.7)}px ${family}`;
		ctx.fillText(input.speaker, px + 20, py + size * 0.7);
		ctx.shadowBlur = 0;
		py += speakerH;
	}
	const ph = panelH;

	if (design === "bubble" || design === "letter" || hasBg) {
		ctx.save();
		ctx.shadowColor = "rgba(0,0,0,0.18)";
		ctx.shadowBlur = 40;
		ctx.globalAlpha = hasBg && design !== "letter" ? 0.9 : 1;
		ctx.fillStyle = c.bg;
		roundRect(ctx, px, py, pw, ph, design === "letter" ? 12 : 40);
		ctx.fill();
		if (design === "bubble") {
			// 꼬리
			ctx.beginPath();
			ctx.moveTo(px + 90, py + ph - 2);
			ctx.lineTo(px + 70, py + ph + tailH);
			ctx.lineTo(px + 160, py + ph - 2);
			ctx.closePath();
			ctx.fill();
		}
		ctx.restore();
	}

	// 디자인별 장식
	if (design === "footnote") {
		ctx.fillStyle = c.accent;
		ctx.fillRect(px + pad - 40, py + pad, 8, ph - pad * 2);
		ctx.globalAlpha = 0.18;
		ctx.textAlign = "left";
		ctx.font = `700 220px Georgia, serif`;
		ctx.fillText("“", px + pw - pad - 120, py + pad + 150);
		ctx.globalAlpha = 1;
	}
	if (design === "letter") {
		ctx.strokeStyle = withAlpha(c.muted, 0.25);
		ctx.lineWidth = 2;
		for (let ly = py + pad + lh; ly < py + ph - pad + 4; ly += lh) {
			ctx.beginPath();
			ctx.moveTo(px + pad, ly + 8);
			ctx.lineTo(px + pw - pad, ly + 8);
			ctx.stroke();
		}
	}

	ctx.textAlign = align;
	const tx = anchor(px + pad, px + pw - pad);
	let y = py + pad + (design === "footnote" ? 30 : 0);
	if (input.speaker && design === "footnote") {
		ctx.fillStyle = c.accent;
		ctx.font = `700 ${Math.round(size * 0.7)}px ${family}`;
		ctx.fillText(input.speaker, tx, y + size * 0.7);
		y += speakerH;
	}
	ctx.fillStyle = c.text;
	ctx.font = `${bold} ${size}px ${family}`;
	for (const line of lines) {
		y += lh;
		ctx.fillText(line, tx, y - (lh - size) / 2);
	}
	// 편지·미니멀: 이름은 글 아래 서명처럼
	if (input.speaker && (design === "letter" || design === "minimal")) {
		y += Math.round(size * 1.3);
		ctx.fillStyle = design === "letter" ? c.text : c.accent;
		ctx.font = `${design === "letter" ? "italic 500" : "700"} ${Math.round(size * 0.66)}px ${family}`;
		ctx.textAlign = design === "letter" ? "right" : align;
		ctx.fillText(`— ${input.speaker}`, design === "letter" ? px + pw - pad : tx, y);
		ctx.textAlign = align;
	}
	if (input.caption) {
		ctx.fillStyle = c.muted;
		ctx.font = `400 28px ${family}`;
		if (design === "letter") {
			ctx.textAlign = "right";
			ctx.fillText(input.caption, px + pw - pad, py + pad - 20);
		} else {
			ctx.fillText(input.caption, tx, py + ph - pad + 20);
		}
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
