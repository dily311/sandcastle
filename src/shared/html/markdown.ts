// GitHub Flavored Markdown → 표시용 HTML (5-E-27, ADR-004 추가).
// - marked(GFM)로 그리고, 줄바꿈은 줄마다(GitHub 댓글처럼 breaks)
// - HTML 렌더링 플러그인이 꺼져 있으면 글 속 태그는 글자로(이스케이프), 켜져 있으면 허용 목록으로 거른다(sanitize.ts)
// - RP 표시: "대사"·“대사” 강조(<q>), `이름: 대사` 말한 사람(다인봇), 작업 목록은 ☐/☑ 글자
// - 결과는 언제나 sanitizeHtml을 거친다 (스크립트·이벤트 속성·javascript: 링크는 남지 않음)
import { Marked, type Tokens } from "marked";
import { sanitizeHtml } from "./sanitize";

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

function makeMarked(allowHtml: boolean) {
	const m = new Marked({ gfm: true, breaks: true, async: false });
	m.use({
		renderer: {
			html(token: Tokens.HTML | Tokens.Tag) {
				if (allowHtml) return token.text;
				// 꺼져 있으면 글자로 — HTML 블록은 줄바꿈을 살린 문단으로
				return token.block ? `<p>${esc(token.text.trim()).replace(/\n/g, "<br>")}</p>\n` : esc(token.text);
			},
			checkbox({ checked }: Tokens.Checkbox) {
				return `<span class="task">${checked ? "☑" : "☐"}</span>`;
			},
		},
	});
	return m;
}
const withHtml = makeMarked(true);
const noHtml = makeMarked(false);

/** 다인봇 (5-F-2): 줄이 `이름: 대사`면 말한 사람 (이름은 24자 안, 지문·대사 기호로 시작하지 않음) */
export const SAY = /^([^:：*"“\s][^:：*"“\n]{0,23})(?::\s+|：\s*)/;

export interface MarkdownOpts {
	/** 글 속 HTML 태그를 그린다 (허용 목록으로 걸러서). 아니면 글자로 */
	allowHtml?: boolean;
	/** `이름: 대사` 줄을 말한 사람으로 */
	speakers?: boolean;
	/** 말한 사람 이름 → 그림 주소 (없으면 이름 첫 글자) */
	avatars?: Record<string, string | null>;
}

/** 말한 사람 줄이 각자 문단이 되게: 그런 줄 앞에 빈 줄 (코드 블록 안은 그대로) */
function splitSpeakerLines(text: string): string {
	let fence = false;
	return text
		.split("\n")
		.map((line, i, all) => {
			if (/^\s*```/.test(line)) fence = !fence;
			const prev = all[i - 1];
			return !fence && i > 0 && prev?.trim() && SAY.test(line.trim()) ? `\n${line}` : line;
		})
		.join("\n");
}

/** 마크다운 글 → 표시용 HTML */
export function renderMarkdown(text: string, opts: MarkdownOpts = {}): string {
	const src = opts.speakers ? splitSpeakerLines(text) : text;
	const raw = (opts.allowHtml ? withHtml : noHtml).parse(src) as string;
	return decorate(sanitizeHtml(raw), opts);
}

/** 코드 블록 밖 HTML(`<details>` 등) — 안의 글도 마크다운으로 그린 뒤 거른다 */
export function renderUiHtml(html: string, opts: MarkdownOpts = {}): string {
	const doc = new DOMParser().parseFromString(`<body>${html}</body>`, "text/html");
	const texts: Text[] = [];
	const walker = doc.createTreeWalker(doc.body, NodeFilter.SHOW_TEXT);
	for (let n = walker.nextNode(); n; n = walker.nextNode()) {
		if (!n.parentElement?.closest("pre, code, style, script, textarea") && n.textContent?.trim()) texts.push(n as Text);
	}
	for (const t of texts) {
		const value = t.textContent ?? "";
		// 여러 줄이면 블록(목록·문단·줄바꿈), 한 줄이면 글자 꾸밈만
		const multi = value.trim().includes("\n");
		const out = multi ? (noHtml.parse(value.trim()) as string) : (noHtml.parseInline(value) as string);
		const tpl = doc.createElement("template");
		tpl.innerHTML = out;
		t.replaceWith(tpl.content);
	}
	return decorate(sanitizeHtml(doc.body.innerHTML), { ...opts, speakers: false });
}

const SPEECH = /"[^"\n]+"|“[^”\n]+”/g;

/** "대사" 강조와 말한 사람 줄 */
function decorate(html: string, opts: MarkdownOpts): string {
	if (!html.includes('"') && !html.includes("“") && !html.includes("&quot;") && !opts.speakers) return html;
	const doc = new DOMParser().parseFromString(`<body>${html}</body>`, "text/html");
	// 대사 강조: 코드·이미 강조한 곳 밖의 글
	const texts: Text[] = [];
	const walker = doc.createTreeWalker(doc.body, NodeFilter.SHOW_TEXT);
	for (let n = walker.nextNode(); n; n = walker.nextNode()) {
		if (!n.parentElement?.closest("pre, code, q") && SPEECH.test(n.textContent ?? "")) texts.push(n as Text);
		SPEECH.lastIndex = 0;
	}
	for (const t of texts) {
		const value = t.textContent ?? "";
		const frag = doc.createDocumentFragment();
		let last = 0;
		for (const m of value.matchAll(SPEECH)) {
			if (m.index! > last) frag.append(value.slice(last, m.index));
			const q = doc.createElement("q");
			q.textContent = m[0];
			frag.append(q);
			last = m.index! + m[0].length;
		}
		if (last < value.length) frag.append(value.slice(last));
		t.replaceWith(frag);
	}
	// 말한 사람: 문단 첫 글이 `이름: `
	if (opts.speakers) {
		for (const p of doc.body.querySelectorAll(":scope > p")) {
			const first = p.firstChild;
			if (!(first instanceof Text)) continue;
			const m = SAY.exec(first.textContent ?? "");
			if (!m) continue;
			const rest = (first.textContent ?? "").slice(m[0].length);
			if (!rest.trim() && !first.nextSibling) continue;
			const name = m[1].trim();
			first.textContent = rest;
			const who = doc.createElement("span");
			who.className = "who sc-speaker";
			const src = opts.avatars?.[name];
			const av = doc.createElement(src ? "img" : "span");
			av.className = "av";
			if (src) av.setAttribute("src", src);
			else av.textContent = [...name][0] ?? "?";
			const b = doc.createElement("b");
			b.textContent = name;
			who.append(av, b);
			p.prepend(who);
			p.classList.add("say");
		}
	}
	return doc.body.innerHTML;
}
