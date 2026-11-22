// 메시지 본문 → 표시용 조각. HTML 문자열을 만들지 않는다 (모두 컴포넌트로 그림).
// - 줄마다 문단, 빈 줄은 버림
// - *지문* → 기울임, "대사" → 대사 강조, **굵게**, `코드`
// - 마크다운 줄: # 제목(######까지), - / * 목록, > 인용, --- 가로줄, | 표 |
// - ``` 코드 블록 (```html은 HTML 블록)
// - <cg>키</cg> → 캐릭터 에셋 (문단 사이 블록)
// - ```html … ``` → HTML 블록 (iframe으로 그림, shared/html/blocks.ts)
import { splitHtmlBlocks } from "$shared/html/blocks";

export type Inline = { kind: "text" | "action" | "speech" | "strong" | "code"; text: string };
export type Block =
	| { kind: "p"; parts: Inline[] }
	| { kind: "h"; level: 1 | 2 | 3; parts: Inline[] }
	| { kind: "li"; parts: Inline[] }
	| { kind: "quote"; parts: Inline[] }
	| { kind: "hr" }
	| { kind: "code"; lang: string; text: string }
	| { kind: "say"; speaker: string; parts: Inline[] }
	| { kind: "table"; head: Inline[][]; rows: Inline[][][] }
	| { kind: "cg"; key: string }
	| { kind: "html"; code: string; /** false: 코드 블록 밖 HTML — 글쓴이의 JS 없이 그림 */ scripts?: false };

const CG = /<cg>\s*([^<]+?)\s*<\/cg>/gi;

function inline(line: string): Inline[] {
	const out: Inline[] = [];
	// `코드` / **굵게** / *지문* / "대사" “대사”
	const re = /(`[^`\n]+`)|(\*\*[^*\n]+\*\*)|(\*[^*\n]+\*)|("[^"\n]+"|“[^”\n]+”)/g;
	let last = 0;
	for (const m of line.matchAll(re)) {
		if (m.index! > last) out.push({ kind: "text", text: line.slice(last, m.index) });
		if (m[1]) out.push({ kind: "code", text: m[1].slice(1, -1) });
		else if (m[2]) out.push({ kind: "strong", text: m[2].slice(2, -2) });
		else if (m[3]) out.push({ kind: "action", text: m[3].slice(1, -1) });
		else out.push({ kind: "speech", text: m[4] });
		last = m.index! + m[0].length;
	}
	if (last < line.length) out.push({ kind: "text", text: line.slice(last) });
	return out;
}

/** 코드 블록 밖에 HTML 태그가 있는가 (캐릭터 소개처럼 HTML로 쓴 글) */
export function hasRawHtml(content: string): boolean {
	return splitHtmlBlocks(content).some((p) => p.kind === "text" && /<(div|p|br|span|img|table|style|h\d|ul|ol|center|font|a|b|i|details)\b[^>]*>/i.test(p.text.replace(CG, "")));
}

/**
 * rawHtml: 코드 블록 밖의 HTML도 HTML 블록으로 그린다 (캐릭터 개요).
 * safeHtml: 메시지에서 태그로 시작하는 줄(태그가 닫힐 때까지)을 **스크립트 없는** HTML 블록으로 (5-E-9, ADR-004 추가).
 * 둘 다 없으면 AI 답변의 HTML은 ```html 블록만 그린다
 */
export function toBlocks(content: string, opts: { rawHtml?: boolean; safeHtml?: boolean; speakers?: boolean } = {}): Block[] {
	if (opts.rawHtml && hasRawHtml(content)) {
		// 글 전체를 한 문서로 (마크다운 줄은 HTML 안에서 의미가 없어 그대로 둠)
		return [{ kind: "html", code: content.replace(/```html\s*\n?|```/gi, "").replace(/\n/g, "<br>\n").replace(/(<br>\n)+(?=<\/?(div|p|ul|ol|li|table|tr|h\d)\b)/gi, "\n") }];
	}
	const blocks = splitHtmlBlocks(content).flatMap((part) =>
		part.kind === "html" ? [{ kind: "html" as const, code: part.code }] : opts.safeHtml ? safeHtmlBlocks(part.text) : textBlocks(part.text),
	);
	return opts.speakers ? blocks.map(asSay) : blocks;
}

/** 다인봇 (5-F-2): 문단이 `이름: 대사`면 말한 사람 블록으로 (이름은 24자 안, 지문·대사 기호로 시작하지 않음) */
const SAY = /^([^:：*"“\s][^:：*"“\n]{0,23})(?::\s+|：\s*)/;
function asSay(b: Block): Block {
	if (b.kind !== "p" || b.parts[0]?.kind !== "text") return b;
	const m = SAY.exec(b.parts[0].text);
	if (!m) return b;
	const rest = b.parts[0].text.slice(m[0].length);
	const parts = rest ? [{ ...b.parts[0], text: rest }, ...b.parts.slice(1)] : b.parts.slice(1);
	return parts.length ? { kind: "say", speaker: m[1].trim(), parts } : b;
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
function safeHtmlBlocks(text: string): Block[] {
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
		if (plain.length) out.push(...textBlocks(plain.join("\n")));
		plain = [];
		out.push({ kind: "html", code: html.join("\n"), scripts: false });
	}
	if (plain.length) out.push(...textBlocks(plain.join("\n")));
	return out;
}

function lineBlock(t: string): Block {
	if (/^(-{3,}|\*{3,}|_{3,})$/.test(t)) return { kind: "hr" };
	const h = /^(#{1,6})\s+(.+)$/.exec(t);
	if (h) return { kind: "h", level: Math.min(h[1].length, 3) as 1 | 2 | 3, parts: inline(h[2].replace(/\s+#+\s*$/, "")) };
	// "* 항목"도 목록 ("*지문*"은 * 바로 뒤에 글자가 붙어서 해당 없음)
	const li = /^[-+*•]\s+(.+)$/.exec(t) ?? /^\d+[.)]\s+(.+)$/.exec(t);
	if (li) return { kind: "li", parts: inline(li[1]) };
	const q = /^>\s?(.*)$/.exec(t);
	if (q) return { kind: "quote", parts: inline(q[1]) };
	return { kind: "p", parts: inline(t) };
}

/** 표의 한 줄 → 칸들 (앞뒤 | 는 떼고, \| 는 글자) */
function cells(line: string): Inline[][] {
	const body = line.trim().replace(/^\|/, "").replace(/(?<!\\)\|$/, "");
	return body.split(/(?<!\\)\|/).map((c) => inline(c.trim().replace(/\\\|/g, "|")));
}
const TABLE_RULE = /^\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?$/;

/** 줄 단위 블록 (표는 여러 줄을 묶음) */
function lineBlocks(text: string): Block[] {
	const out: Block[] = [];
	const lines = text.split(/\n/);
	for (let i = 0; i < lines.length; i++) {
		const t = lines[i].trim();
		if (!t) continue;
		if (t.startsWith("|") && TABLE_RULE.test(lines[i + 1]?.trim() ?? "")) {
			const head = cells(t);
			const rows: Inline[][][] = [];
			i += 1;
			while (i + 1 < lines.length && lines[i + 1].trim().startsWith("|")) rows.push(cells(lines[++i]));
			out.push({ kind: "table", head, rows });
			continue;
		}
		out.push(lineBlock(t));
	}
	return out;
}

/** ``` 코드 블록 (닫히지 않았으면 끝까지 — 받는 중인 응답) */
const FENCE = /```([\w+-]*)[^\n]*\n([\s\S]*?)(?:\n?```|$)/g;

function textBlocks(content: string): Block[] {
	const blocks: Block[] = [];
	// <cg>를 따로 떼어내 문단 사이에 둔다
	const chunks = content.split(CG);
	chunks.forEach((chunk, i) => {
		if (i % 2 === 1) {
			blocks.push({ kind: "cg", key: chunk.trim() });
			return;
		}
		let last = 0;
		for (const m of chunk.matchAll(FENCE)) {
			blocks.push(...lineBlocks(chunk.slice(last, m.index)));
			blocks.push({ kind: "code", lang: m[1], text: m[2] });
			last = m.index! + m[0].length;
		}
		blocks.push(...lineBlocks(chunk.slice(last)));
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
