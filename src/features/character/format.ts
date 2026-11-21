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
	| { kind: "table"; head: Inline[][]; rows: Inline[][][] }
	| { kind: "cg"; key: string }
	| { kind: "html"; code: string };

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
 * 메시지는 기본값(false) — AI 답변에서는 ```html 블록만 실행한다 (ADR-004)
 */
export function toBlocks(content: string, opts: { rawHtml?: boolean } = {}): Block[] {
	if (opts.rawHtml && hasRawHtml(content)) {
		// 글 전체를 한 문서로 (마크다운 줄은 HTML 안에서 의미가 없어 그대로 둠)
		return [{ kind: "html", code: content.replace(/```html\s*\n?|```/gi, "").replace(/\n/g, "<br>\n").replace(/(<br>\n)+(?=<\/?(div|p|ul|ol|li|table|tr|h\d)\b)/gi, "\n") }];
	}
	return splitHtmlBlocks(content).flatMap((part) => (part.kind === "html" ? [{ kind: "html" as const, code: part.code }] : textBlocks(part.text)));
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
