/** 선택한 DOM 범위에서 본문 텍스트를 추출한다 (<br> 및 블록 요소 경계의 줄바꿈을 보존) */
export function extractRangeText(block: HTMLElement, range: Range, skip: (el: Element | null) => boolean): string {
	const BLOCK_SEL = "p, h1, h2, h3, h4, h5, h6, li, blockquote, pre, div";
	const blockOf = (el: Element | null): Element | null => {
		const b = el?.closest(BLOCK_SEL) ?? null;
		if (!b || b === block || b.classList.contains("md") || b.classList.contains("body") || b.classList.contains("content")) return null;
		return b;
	};
	let text = "";
	let lastLine: Element | null = null;
	const walker = document.createTreeWalker(block, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
	for (let n = walker.nextNode(); n; n = walker.nextNode()) {
		if (!range.intersectsNode(n)) continue;
		if (n.nodeType === Node.ELEMENT_NODE) {
			const elem = n as Element;
			if (elem.tagName === "BR" && !skip(elem)) {
				const brRange = document.createRange();
				brRange.selectNode(elem);
				const startsBeforeEnd = range.compareBoundaryPoints(Range.START_TO_END, brRange) >= 0;
				const endsAfterStart = range.compareBoundaryPoints(Range.END_TO_START, brRange) <= 0;
				if (startsBeforeEnd && endsAfterStart && text.length > 0) text += "\n";
			}
			continue;
		}
		if (skip(n.parentElement)) continue;
		const raw = n.textContent ?? "";
		const s = n === range.startContainer ? range.startOffset : 0;
		const e = n === range.endContainer ? range.endOffset : raw.length;
		const chunk = raw.slice(s, e);
		if (!chunk) continue;
		const line = blockOf(n.parentElement);
		if (lastLine && line && line !== lastLine && text.length > 0 && !text.endsWith("\n")) text += "\n";
		if (line) lastLine = line;
		text += chunk;
	}
	return text.trim();
}
