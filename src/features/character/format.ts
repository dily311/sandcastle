// 메시지 본문 → 표시용 조각 (5-E-27: GitHub Flavored Markdown).
// - 글은 marked(GFM)로 그린 HTML(허용 목록으로 거름) — 링크·목록·표·취소선·작업 목록·코드 블록 등
// - RP 표시: *지문* → 기울임, "대사" → 대사 강조, `이름: 대사` → 말한 사람 (shared/html/markdown.ts)
// - <cg>키</cg> → 캐릭터 에셋 (문단 사이 블록)
// - ```html … ``` → HTML 블록 (iframe으로 그림, JS 허용, shared/html/blocks.ts)
// - 코드 블록 밖 HTML 줄(HTML 렌더링 플러그인이 켜졌을 때) → 메시지 안 UI, 안의 글도 마크다운 (renderUiHtml)
import { splitHtmlBlocks, wholeHtmlBlock } from "$shared/html/blocks";
import { renderMarkdown, SAY } from "$shared/html/markdown";

export { SAY };

export type Block =
	/** 마크다운을 그린 HTML (이미 거름) */
	| { kind: "md"; html: string }
	| { kind: "cg"; key: string }
	| {
			kind: "html";
			code: string;
			/** false: 코드 블록 밖 HTML — 글쓴이의 JS 없이 메시지 안에 그림 */
			scripts?: false;
			/** 상태창 출처 키 (코드 블록 감싸기가 벗겨진 경우에도 출처 유지) */
			pin?: string;
	  };

const CG = /<cg>\s*([^<]+?)\s*<\/cg>/gi;

interface Opts {
	rawHtml?: boolean;
	safeHtml?: boolean;
	speakers?: boolean;
	/** 말한 사람 이름 → 그림 주소 (다인봇) */
	avatars?: Record<string, string | null>;
}

/** 코드 블록 밖에 HTML 태그가 있는가 (캐릭터 소개처럼 HTML로 쓴 글) */
export function hasRawHtml(content: string): boolean {
	return splitHtmlBlocks(content).some((p) => p.kind === "text" && /<(div|p|br|span|img|table|style|h\d|ul|ol|center|font|a|b|i|details)\b[^>]*>/i.test(p.text.replace(CG, "")));
}

const PIN_END_MARKER = "<!--/sc-pin-->";
const PIN_OPEN_RE =
	/<([a-z][\w-]*)\b(?:[^>"']|"[^"]*"|'[^']*')*?\bdata-sc-pin\s*=\s*(?:"(status(?::[^"]*)?)"|'(status(?::[^']*)?)'|(status(?::[^"'\s>]*)?))(?:[^>"']|"[^"]*"|'[^']*')*?>/gi;
const ANY_FENCE_RE = /```[ \t]*([\w-]*)[ \t]*\r?\n([\s\S]*?)\r?\n?```/g;

/** 열린 상태창 태그 이후에서 닫는 위치를 찾는다 (주석·코드블록 안 태그는 건너뛰고, 닫히지 않은 태그가 있어도 다음 상태창을 삼키지 않음) */
function findPinEnd(text: string, openEnd: number, tag: string): { innerEnd: number; fullEnd: number; afterEnd: number } {
	const markerIdx = text.indexOf(PIN_END_MARKER, openEnd);
	if (markerIdx !== -1 && !/\bdata-sc-pin\s*=\s*["']?status\b/i.test(text.slice(openEnd, markerIdx))) {
		const sub = text.slice(openEnd, markerIdx);
		const closeRe = new RegExp(`<\\/${tag}\\s*>\\s*$`, "i");
		const cm = closeRe.exec(sub);
		if (cm) {
			return {
				innerEnd: openEnd + cm.index,
				fullEnd: openEnd + cm.index + cm[0].trimEnd().length,
				afterEnd: markerIdx + PIN_END_MARKER.length,
			};
		}
	}
	const scan = /<!--[\s\S]*?-->|```[\s\S]*?```|<(\/?)([a-z][\w-]*)\b((?:[^>"']|"[^"]*"|'[^']*')*?)(\/?)>/gi;
	scan.lastIndex = openEnd;
	let depth = 1;
	for (let t = scan.exec(text); t; t = scan.exec(text)) {
		if (t[0].startsWith("<!--") || t[0].startsWith("```")) continue;
		const isClose = t[1] === "/";
		const tName = t[2].toLowerCase();
		const attrs = t[3];
		const selfClose = t[4] === "/";
		if (!isClose && /\bdata-sc-pin\s*=\s*["']?status\b/i.test(attrs)) {
			return { innerEnd: t.index, fullEnd: t.index, afterEnd: t.index };
		}
		if (tName !== tag || VOID.has(tName) || selfClose) continue;
		depth += isClose ? -1 : 1;
		if (!depth) {
			const fullEnd = t.index + t[0].length;
			const afterEnd = text.slice(fullEnd).startsWith(PIN_END_MARKER) ? fullEnd + PIN_END_MARKER.length : fullEnd;
			return { innerEnd: t.index, fullEnd, afterEnd };
		}
	}
	return { innerEnd: text.length, fullEnd: text.length, afterEnd: text.length };
}

/**
 * 상태창 표시(`data-sc-pin="status"`, 5-F-4-2)가 붙은 요소를 제 줄로 떼어 둔다 — 문단 가운데 있어도 따로 된 HTML 블록이 되게.
 */
export function isolatePins(text: string): string {
	return splitPins(text)
		.map((p) => (p.pin ? `\n\n${p.text}\n\n` : p.text))
		.join("")
		.replace(/[ \t]+\n\n/g, "\n\n")
		.replace(/\n\n[ \t]+/g, "\n\n")
		.replace(/\n{3,}/g, "\n\n");
}

/** 글을 상태창 요소(pin)와 나머지로 나눈다 (순서대로) */
function splitPins(text: string): { pin: boolean; text: string; inner?: string; key?: string }[] {
	if (!/data-sc-pin/i.test(text)) return [{ pin: false, text }];
	const open = new RegExp(PIN_OPEN_RE.source, "gi");
	const out: { pin: boolean; text: string; inner?: string; key?: string }[] = [];
	let at = 0;
	for (let m = open.exec(text); m; m = open.exec(text)) {
		if (m.index < at) continue;
		const tag = m[1].toLowerCase();
		const key = m[2] ?? m[3] ?? m[4] ?? "status";
		const openEnd = m.index + m[0].length;
		const { innerEnd, fullEnd, afterEnd } = findPinEnd(text, openEnd, tag);
		if (m.index > at) out.push({ pin: false, text: text.slice(at, m.index) });
		out.push({ pin: true, text: text.slice(m.index, fullEnd), inner: text.slice(openEnd, innerEnd), key });
		at = afterEnd;
		open.lastIndex = afterEnd;
	}
	if (at < text.length) out.push({ pin: false, text: text.slice(at) });
	return out;
}

/** 상태창 요소 하나를 블록으로 변환 (안쪽이 ```html 코드 블록이면 iframe 블록으로 풀고 출처 키를 보존) */
function pinToBlocks(full: string, inner: string, key: string): Block[] {
	if (!inner.trim()) return [];
	const fenced = wholeHtmlBlock(inner);
	if (fenced !== null) return [{ kind: "html", code: fenced, pin: key }];
	const parts = splitHtmlBlocks(inner);
	if (parts.some((p) => p.kind === "html")) {
		return parts.flatMap((p): Block[] =>
			p.kind === "html"
				? [{ kind: "html", code: p.code, pin: key }]
				: p.text.trim()
					? [{ kind: "html", code: p.text, scripts: false, pin: key }]
					: [],
		);
	}
	return [{ kind: "html", code: full, scripts: false }];
}

function plainToBlocks(text: string, opts: Opts): Block[] {
	if (!text.trim()) return [];
	return opts.safeHtml ? safeHtmlBlocks(text, opts) : textBlocks(text, opts);
}

/**
 * rawHtml: 코드 블록 밖의 HTML도 HTML 블록으로 그린다 (캐릭터 개요).
 * safeHtml: 메시지에서 태그로 시작하는 줄(태그가 닫힐 때까지)을 **스크립트 없는** HTML 블록으로 (5-E-9, ADR-004 추가).
 * 둘 다 없으면 AI 답변의 HTML은 ```html 블록만 그린다.
 * 상태창 요소(data-sc-pin="status...")는 safeHtml 여부·안쪽 코드블록 여부와 무관하게 온전히 상태창 블록으로 분리한다.
 */
export function toBlocks(content: string, opts: Opts = {}): Block[] {
	if (opts.rawHtml && hasRawHtml(content)) {
		// 글 전체를 한 문서로 (마크다운 줄은 HTML 안에서 의미가 없어 그대로 둠)
		return [{ kind: "html", code: content.replace(/```html\s*\n?|```/gi, "").replace(/\n/g, "<br>\n").replace(/(<br>\n)+(?=<\/?(div|p|ul|ol|li|table|tr|h\d)\b)/gi, "\n") }];
	}
	if (!content.includes("```") && !/data-sc-pin/i.test(content)) {
		return plainToBlocks(content, opts);
	}
	const out: Block[] = [];
	const fenceRe = new RegExp(ANY_FENCE_RE.source, "g");
	const pinRe = new RegExp(PIN_OPEN_RE.source, "gi");
	let plainFrom = 0;
	let scanPos = 0;
	while (scanPos < content.length) {
		fenceRe.lastIndex = scanPos;
		pinRe.lastIndex = scanPos;
		const fm = fenceRe.exec(content);
		const pm = pinRe.exec(content);
		if (!fm && !pm) break;
		if (fm && (!pm || fm.index <= pm.index)) {
			const html = wholeHtmlBlock(fm[0]);
			const fenceEnd = fm.index + fm[0].length;
			if (html !== null) {
				out.push(...plainToBlocks(content.slice(plainFrom, fm.index), opts));
				out.push({ kind: "html", code: html });
				plainFrom = fenceEnd;
			}
			scanPos = fenceEnd;
		} else if (pm) {
			out.push(...plainToBlocks(content.slice(plainFrom, pm.index), opts));
			const tag = pm[1].toLowerCase();
			const key = pm[2] ?? pm[3] ?? pm[4] ?? "status";
			const openEnd = pm.index + pm[0].length;
			const { innerEnd, fullEnd, afterEnd } = findPinEnd(content, openEnd, tag);
			out.push(...pinToBlocks(content.slice(pm.index, fullEnd), content.slice(openEnd, innerEnd), key));
			plainFrom = afterEnd;
			scanPos = afterEnd;
		}
	}
	if (plainFrom < content.length) {
		out.push(...plainToBlocks(content.slice(plainFrom), opts));
	}
	return out;
}

/** HTML로 여는 태그 (줄 맨 앞에 오면 그 줄부터 HTML 덩어리) */
const HTML_START = /^<(div|p|table|ul|ol|details|section|article|aside|center|span|img|style|h[1-6]|font|figure|blockquote|pre|hr|br|b|i|a|small|strong|em|u|s|sup|sub)\b/i;
const VOID = new Set(["br", "hr", "img", "input", "meta", "link", "source", "wbr", "col", "area"]);
/** 줄의 여닫는 태그 차이 (빈 요소·스스로 닫는 태그는 세지 않음) */
function depthOf(line: string): number {
	let d = 0;
	for (const m of line.matchAll(/<(\/?)([a-z][\w-]*)\b[^>]*?(\/?)>/gi)) {
		if (VOID.has(m[2].toLowerCase()) || m[3]) continue;
		d += m[1] ? -1 : 1;
	}
	return d;
}

/** 글과 HTML 덩어리를 나눈다: 태그로 시작하는 줄부터 태그가 모두 닫히는 줄까지가 HTML */
function safeHtmlBlocks(text: string, opts: Opts): Block[] {
	const out: Block[] = [];
	const lines = text.split("\n");
	let plain: string[] = [];
	for (let i = 0; i < lines.length; i++) {
		if (!HTML_START.test(lines[i].trim())) {
			plain.push(lines[i]);
			continue;
		}
		let depth = 0;
		const html: string[] = [];
		for (; i < lines.length; i++) {
			html.push(lines[i]);
			depth += depthOf(lines[i]);
			if (depth <= 0) break;
		}
		if (plain.length) out.push(...textBlocks(plain.join("\n"), opts));
		plain = [];
		out.push({ kind: "html", code: html.join("\n"), scripts: false });
	}
	if (plain.length) out.push(...textBlocks(plain.join("\n"), opts));
	return out;
}

/** 글 → 마크다운 블록 (<cg>는 따로 떼어 문단 사이에) */
function textBlocks(content: string, opts: Opts): Block[] {
	const blocks: Block[] = [];
	content.split(CG).forEach((chunk, i) => {
		if (i % 2 === 1) return void blocks.push({ kind: "cg", key: chunk.trim() });
		if (!chunk.trim()) return;
		const html = renderMarkdown(chunk, { allowHtml: opts.safeHtml, speakers: opts.speakers, avatars: opts.avatars });
		if (html.trim()) blocks.push({ kind: "md", html });
	});
	return blocks;
}

/** 목차·검색용 한 줄 요약 */
export function preview(content: string, max = 40): string {
	const t = content.replace(CG, "").replace(/[*"“”`#>]/g, "").replace(/\s+/g, " ").trim();
	return t.length > max ? `${t.slice(0, max)}…` : t;
}

/** {{char}} / {{user}} 이름 바꾸기 (화면용) */
export function fillNames(text: string, names: { char?: string; user?: string }): string {
	return text.replace(/\{\{\s*(char|user)\s*\}\}/gi, (m, k: string) => (k.toLowerCase() === "char" ? names.char : names.user) || m);
}
