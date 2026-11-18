import { describe, expect, it } from "vitest";
import { findTextRange } from "./highlight";

function body(html: string) {
	const el = document.createElement("div");
	el.innerHTML = html;
	document.body.appendChild(el);
	return el;
}

describe("findTextRange", () => {
	it("한 문단 안의 글을 찾는다", () => {
		const el = body("<p>안녕, 오늘은 날씨가 좋네요.</p>");
		expect(findTextRange(el, "오늘은 날씨")?.toString()).toBe("오늘은 날씨");
	});
	it("여러 글 노드(행동·대사 표시)에 걸쳐도 찾는다", () => {
		const el = body("<p><em>웃으며</em> <q>어서 와</q> 라고 말했다.</p>");
		expect(findTextRange(el, "웃으며 어서 와 라고")?.toString()).toBe("웃으며 어서 와 라고");
	});
	it("문단을 넘어 고른 글(줄바꿈)도 찾는다", () => {
		const el = body("<p>첫 문단 끝.</p><p>둘째 문단 시작.</p>");
		const r = findTextRange(el, "끝.\n\n둘째");
		expect(r).not.toBeNull();
		expect(r?.startContainer.textContent).toBe("첫 문단 끝.");
		expect(r?.endContainer.textContent).toBe("둘째 문단 시작.");
	});
	it("머리줄·버튼 글은 본문으로 치지 않는다", () => {
		const el = body('<div class="who"><b>나</b></div><p>본문</p>');
		expect(findTextRange(el, "나본문")).toBeNull();
		expect(findTextRange(el, "본문")?.toString()).toBe("본문");
	});
	it("없는 글은 null", () => {
		expect(findTextRange(body("<p>abc</p>"), "xyz")).toBeNull();
		expect(findTextRange(body("<p>abc</p>"), "  ")).toBeNull();
	});
});
