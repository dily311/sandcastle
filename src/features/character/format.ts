// 메시지 본문 → 표시용 조각. HTML은 쓰지 않는다 (모두 텍스트로 그림).
// - 빈 줄/줄바꿈으로 문단 나누기
// - *지문* → 기울임, "대사" → 대사 강조
// - <cg>키</cg> → 캐릭터 이미지 (문단 사이 블록)
// - ```html … ``` → HTML 블록 (iframe으로 그림, shared/html/blocks.ts)
import { splitHtmlBlocks } from "$shared/html/blocks";

export type Inline = { kind: "text" | "action" | "speech"; text: string };
export type Block = { kind: "p"; parts: Inline[] } | { kind: "cg"; key: string } | { kind: "html"; code: string };

const CG = /<cg>\s*([^<]+?)\s*<\/cg>/gi;

function inline(line: string): Inline[] {
	const out: Inline[] = [];
	// *...* 또는 "..." / “...”
	const re = /(\*[^*\n]+\*)|("[^"\n]+"|“[^”\n]+”)/g;
	let last = 0;
	for (const m of line.matchAll(re)) {
		if (m.index! > last) out.push({ kind: "text", text: line.slice(last, m.index) });
		if (m[1]) out.push({ kind: "action", text: m[1].slice(1, -1) });
		else out.push({ kind: "speech", text: m[2] });
		last = m.index! + m[0].length;
	}
	if (last < line.length) out.push({ kind: "text", text: line.slice(last) });
	return out;
}

export function toBlocks(content: string): Block[] {
	return splitHtmlBlocks(content).flatMap((part) => (part.kind === "html" ? [{ kind: "html" as const, code: part.code }] : textBlocks(part.text)));
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
			if (t) blocks.push({ kind: "p", parts: inline(t) });
		}
	});
	return blocks;
}

/** 목차·검색용 한 줄 요약 */
export function preview(content: string, max = 40): string {
	const t = content.replace(CG, "").replace(/[*"“”]/g, "").replace(/\s+/g, " ").trim();
	return t.length > max ? `${t.slice(0, max)}…` : t;
}
