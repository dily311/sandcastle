// AI 답변 안의 ``` 코드 블록 중 HTML을 찾는다. 화면은 HTML 블록을 HtmlView(iframe)로 그린다.
// - ```html … ``` (언어 표시가 html/htm)
// - 언어 표시가 없는 ``` … ``` 인데 내용이 태그로 시작하는 것
// 닫히지 않은 블록(스트리밍 중)은 그대로 글로 둔다.

export type TextOrHtml = { kind: "text"; text: string } | { kind: "html"; code: string };

const FENCE = /```[ \t]*([\w-]*)[ \t]*\r?\n([\s\S]*?)\r?\n?```/g;

function htmlCode(lang: string, body: string): string | null {
	const code = body.trim();
	const l = lang.toLowerCase();
	if (l === "html" || l === "htm") return code;
	if (!l && /^<(!doctype|[a-z])/i.test(code)) return code;
	return null;
}

/** 글을 HTML 블록과 나머지 글로 나눈다 (순서 유지, 빈 글 조각은 뺌) */
export function splitHtmlBlocks(text: string): TextOrHtml[] {
	const out: TextOrHtml[] = [];
	let last = 0;
	for (const m of text.matchAll(FENCE)) {
		const code = htmlCode(m[1] ?? "", m[2] ?? "");
		if (code === null) continue;
		const before = text.slice(last, m.index);
		if (before.trim()) out.push({ kind: "text", text: before });
		out.push({ kind: "html", code });
		last = m.index! + m[0].length;
	}
	const rest = text.slice(last);
	if (rest.trim() || !out.length) out.push({ kind: "text", text: rest });
	return out;
}

/** 글 전체가 HTML 블록 하나면 그 코드 (게임 대사 한 줄 등) */
export function wholeHtmlBlock(text: string): string | null {
	const parts = splitHtmlBlocks(text.trim());
	return parts.length === 1 && parts[0].kind === "html" ? parts[0].code : null;
}
