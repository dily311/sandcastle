// 북마크한 글을 본문에서 찾아 형광펜처럼 칠한다.
// 본문을 고치지 않고 CSS Custom Highlight API(::highlight(bookmark))로 범위만 칠하므로 글·복사·선택에 영향이 없다.
// 칠하는 색은 shared/ui/base.css의 ::highlight(bookmark).

const BLOCK = "p, blockquote, h2, h3, h4, li, figcaption";
const SKIP = ".who, .actions, .swipe, .tr-note, .fold, .folded-line, button, textarea, input";

interface Flat {
	text: string;
	at: { node: Text; offset: number }[];
}

/** 본문 글을 공백 하나로 합쳐 한 줄로 만들고, 각 글자가 어느 글 노드의 몇 번째인지 함께 돌려준다 */
function flatten(root: Element): Flat {
	const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
	const text: string[] = [];
	const at: Flat["at"] = [];
	const push = (c: string, node: Text, offset: number) => {
		text.push(c);
		at.push({ node, offset });
	};
	let prevBlock: Element | null = null;
	for (let n = walker.nextNode() as Text | null; n; n = walker.nextNode() as Text | null) {
		const parent = n.parentElement;
		if (!parent || parent.closest(SKIP)) continue;
		// 문단이 바뀌면 고른 글(selection.toString)에는 줄바꿈이 들어 있으므로 공백 하나를 끼움
		const block = parent.closest(BLOCK);
		if (text.length && text[text.length - 1] !== " " && block !== prevBlock) push(" ", n, 0);
		prevBlock = block;
		const data = n.data;
		for (let i = 0; i < data.length; i++) {
			if (/\s/.test(data[i])) {
				if (text.length && text[text.length - 1] !== " ") push(" ", n, i);
			} else push(data[i], n, i);
		}
	}
	return { text: text.join(""), at };
}

const squash = (s: string) => s.replace(/\s+/g, " ").trim();

/** 본문(root)에서 text가 있는 자리를 Range로. 못 찾으면 null (글이 고쳐졌거나 다른 스와이프) */
export function findTextRange(root: Element, text: string): Range | null {
	const needle = squash(text);
	if (!needle) return null;
	const flat = flatten(root);
	const i = flat.text.indexOf(needle);
	if (i < 0) return null;
	const a = flat.at[i];
	const b = flat.at[i + needle.length - 1];
	const range = document.createRange();
	range.setStart(a.node, a.offset);
	range.setEnd(b.node, b.offset + 1);
	return range;
}

// ---- 칠하기: 메시지마다 범위를 두고 한 Highlight로 묶어 등록 ----
const byOwner = new Map<string, Range[]>();
const supported = () => typeof CSS !== "undefined" && "highlights" in CSS && typeof Highlight !== "undefined";

function apply() {
	if (!supported()) return;
	const all = [...byOwner.values()].flat();
	if (all.length) CSS.highlights.set("bookmark", new Highlight(...all));
	else CSS.highlights.delete("bookmark");
}

/** owner(메시지 id)의 북마크 범위를 바꿔 칠한다. 빈 배열이면 지움 */
export function paintBookmarks(owner: string, ranges: Range[]) {
	if (ranges.length) byOwner.set(owner, ranges);
	else byOwner.delete(owner);
	apply();
}
