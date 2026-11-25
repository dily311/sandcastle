// 코드 블록 밖 HTML을 메시지 안에 바로 그릴 때 거르는 허용 목록 (ADR-004 추가, 2026-11-22).
// - DOMParser로 읽는다: 읽기만 하고 스크립트는 돌지 않는다
// - 허용한 태그·속성만 남긴다. 스크립트·스타일 시트·iframe·폼은 내용째 버리고, 모르는 태그는 껍데기만 벗긴다
// - 링크는 앱 창을 다른 곳으로 보내지 않도록 글자(span)로 바꾼다. 그림은 https·data 그림만
// - ```html 코드 블록은 여기를 거치지 않는다 (iframe에서 JS까지 허용, HtmlView)

const ALLOWED = new Set([
	"details", "summary", "div", "span", "p", "br", "hr", "b", "i", "u", "s", "del", "ins", "kbd", "strong", "em", "small", "big", "mark", "sup", "sub",
	"h1", "h2", "h3", "h4", "h5", "h6", "blockquote", "pre", "code", "center", "font", "ul", "ol", "li", "dl", "dt", "dd",
	"table", "thead", "tbody", "tfoot", "tr", "th", "td", "caption", "colgroup", "col", "img", "figure", "figcaption", "ruby", "rt", "rp", "abbr", "q", "cite",
]);
/** 내용째 버리는 태그 */
const DROP = new Set(["script", "style", "iframe", "frame", "frameset", "object", "embed", "link", "meta", "base", "form", "input", "button", "textarea", "select", "option", "svg", "math", "noscript", "template", "audio", "video", "source", "canvas", "dialog", "portal"]);
const ATTRS = new Set(["class", "title", "style", "open", "colspan", "rowspan", "align", "valign", "width", "height", "color", "face", "size", "alt", "src", "start", "type", "lang", "dir"]);

/** 위험하거나 앱 화면을 덮을 수 있는 CSS 선언을 뺀다 */
function cleanStyle(style: string): string {
	return style
		.split(";")
		.map((d) => d.trim())
		.filter((d) => d && !/url\s*\(|expression|behavior|-moz-binding|javascript:/i.test(d) && !/^position\s*:\s*(fixed|sticky)/i.test(d))
		.join("; ");
}

function cleanNode(node: Element, doc: Document) {
	for (const child of [...node.children]) {
		const tag = child.tagName.toLowerCase();
		if (DROP.has(tag)) {
			child.remove();
			continue;
		}
		cleanNode(child, doc);
		if (tag === "a") {
			// 링크는 글자로 (앱 창이 그 주소로 넘어가지 않게). 주소는 툴팁으로
			const span = doc.createElement("span");
			const href = child.getAttribute("href");
			if (href && /^https?:/i.test(href.trim())) span.setAttribute("title", href);
			span.className = "html-link";
			span.append(...child.childNodes);
			child.replaceWith(span);
			continue;
		}
		if (!ALLOWED.has(tag)) {
			child.replaceWith(...child.childNodes);
			continue;
		}
		for (const attr of [...child.attributes]) {
			const name = attr.name.toLowerCase();
			if (!ATTRS.has(name)) child.removeAttribute(attr.name);
			else if (name === "style") child.setAttribute("style", cleanStyle(attr.value));
			else if (name === "src" && !(tag === "img" && /^(https:|data:image\/)/i.test(attr.value.trim()))) child.removeAttribute(attr.name);
		}
		if (tag === "img") child.setAttribute("loading", "lazy");
	}
}

/** 허용 목록으로 거른 HTML 문자열 */
export function sanitizeHtml(html: string): string {
	const doc = new DOMParser().parseFromString(`<body>${html}</body>`, "text/html");
	cleanNode(doc.body, doc);
	return doc.body.innerHTML;
}

/** 글의 줄바꿈을 화면에서도 줄바꿈으로 (블록 태그 앞뒤는 빼서 빈 줄이 생기지 않게) */
export function htmlWithBreaks(html: string): string {
	const BLOCK = "div|p|ul|ol|li|table|thead|tbody|tr|td|th|h\\d|details|summary|blockquote|pre|hr|dl|dt|dd|figure|center";
	return html
		.trim()
		.replace(/\n/g, "<br>\n")
		.replace(new RegExp(`(<br>\\n)+(?=\\s*<\\/?(${BLOCK})\\b)`, "gi"), "\n")
		.replace(new RegExp(`(<\\/?(${BLOCK})\\b[^>]*>)\\s*<br>\\n`, "gi"), "$1\n");
}
