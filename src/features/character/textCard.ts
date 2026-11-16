// 고른 글을 이미지 카드(png)로. 지금 테마 색을 쓰고 캔버스에 직접 그린다 (외부 라이브러리 없음).

export type CardStyle = "theme" | "paper" | "night";

export interface CardInput {
	text: string;
	/** 말한 사람 (메시지 이름) */
	speaker?: string;
	/** 아래 작은 글씨 (캐릭터 · 대화 제목) */
	caption?: string;
	style: CardStyle;
}

const W = 1080;
const PAD = 96;

function cssVar(name: string, fallback: string) {
	const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	return v || fallback;
}

function palette(style: CardStyle) {
	if (style === "paper") return { bg: "#f6f1e7", text: "#2b2620", muted: "#8a7f70", accent: "#b0643a" };
	if (style === "night") return { bg: "#14161c", text: "#e9eaf0", muted: "#8b90a0", accent: "#8ab4ff" };
	return {
		bg: cssVar("--surface", "#ffffff"),
		text: cssVar("--text", "#1a1a1a"),
		muted: cssVar("--text-muted", "#666666"),
		accent: cssVar("--accent", "#5b6cff"),
	};
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

/** 카드를 그린 캔버스 */
export function renderCard(input: CardInput): HTMLCanvasElement {
	const c = palette(input.style);
	const font = cssVar("--font-sans", "system-ui, sans-serif");
	const text = input.text.trim().slice(0, 2000);
	// 글이 길수록 글자를 조금 작게
	const size = text.length > 600 ? 34 : text.length > 250 ? 40 : 48;
	const lh = Math.round(size * 1.6);

	const measure = document.createElement("canvas").getContext("2d")!;
	measure.font = `500 ${size}px ${font}`;
	const lines = wrapLines(measure, text, W - PAD * 2);

	const head = input.speaker ? 72 : 0;
	const foot = input.caption ? 96 : 40;
	const H = Math.max(W * 0.6, PAD * 2 + 60 + head + lines.length * lh + foot);

	const canvas = document.createElement("canvas");
	const scale = 1;
	canvas.width = W * scale;
	canvas.height = Math.ceil(H) * scale;
	const ctx = canvas.getContext("2d")!;
	ctx.scale(scale, scale);

	ctx.fillStyle = c.bg;
	ctx.fillRect(0, 0, W, H);
	// 왼쪽 강조 막대와 큰 따옴표
	ctx.fillStyle = c.accent;
	ctx.fillRect(PAD - 40, PAD, 8, H - PAD * 2);
	ctx.globalAlpha = 0.18;
	ctx.font = `700 220px Georgia, serif`;
	ctx.fillText("“", W - PAD - 120, PAD + 150);
	ctx.globalAlpha = 1;

	let y = PAD + 30;
	if (input.speaker) {
		ctx.fillStyle = c.accent;
		ctx.font = `700 ${Math.round(size * 0.7)}px ${font}`;
		ctx.fillText(input.speaker, PAD, y + size * 0.7);
		y += head;
	}
	ctx.fillStyle = c.text;
	ctx.font = `500 ${size}px ${font}`;
	ctx.textBaseline = "alphabetic";
	for (const line of lines) {
		y += lh;
		ctx.fillText(line, PAD, y - (lh - size) / 2);
	}
	if (input.caption) {
		ctx.fillStyle = c.muted;
		ctx.font = `400 28px ${font}`;
		ctx.fillText(input.caption, PAD, H - PAD + 10);
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
