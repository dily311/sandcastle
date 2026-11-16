// 메시지 본문 → 표시용 조각. HTML 문자열을 만들지 않는다 (모두 컴포넌트로 그림).
// - 줄마다 문단, 빈 줄은 버림
// - *지문* → 기울임, "대사" → 대사 강조, **굵게**, `코드`
// - 마크다운 줄: # 제목, - 목록, > 인용, --- 가로줄
// - <cg>키</cg> → 캐릭터 이미지 (문단 사이 블록)
// - ```html … ``` → HTML 블록 (iframe으로 그림, shared/html/blocks.ts)
import { splitHtmlBlocks } from "$shared/html/blocks";

export type Inline = { kind: "text" | "action" | "speech" | "strong" | "code"; text: string };
export type Block =
	| { kind: "p"; parts: Inline[] }
	| { kind: "h"; level: 1 | 2 | 3; parts: Inline[] }
	| { kind: "li"; parts: Inline[] }
	| { kind: "quote"; parts: Inline[] }
	| { kind: "hr" }
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
	const h = /^(#{1,3})\s+(.+)$/.exec(t);
	if (h) return { kind: "h", level: h[1].length as 1 | 2 | 3, parts: inline(h[2]) };
	const li = /^[-+•]\s+(.+)$/.exec(t) ?? /^\d+[.)]\s+(.+)$/.exec(t);
	if (li) return { kind: "li", parts: inline(li[1]) };
	const q = /^>\s?(.*)$/.exec(t);
	if (q) return { kind: "quote", parts: inline(q[1]) };
	return { kind: "p", parts: inline(t) };
}

function textBlocks(content: string): Block[] {
	const blocks: Block[] = [];
	// <cg>를 따로 떼어내 문단 사이에 둔다
	const chunks = content.split(CG);
	chunks.forEach((chunk, i) => {
		if (i % 2 === 1) {
			blocks.push({ kind: "cg", key: chunk.trim() });
			return;
		}
		for (const para of chunk.split(/\n/)) {
			const t = para.trim();
			if (t) blocks.push(lineBlock(t));
		}
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
