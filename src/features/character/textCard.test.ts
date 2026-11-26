import { describe, expect, it } from "vitest";
import { maskText } from "./textCard";
import { extractRangeText } from "./selectionText";

describe("maskText (5-E-11)", () => {
	it("숨길 말은 같은 길이의 ●로, 대체어는 바꿔서", () => {
		expect(maskText("이런 젠장, Damn 아리아!", { hide: "젠장, damn", replace: "아리아=A" })).toBe("이런 ●●, ●●●● A!");
	});
	it("긴 말부터, 빈 칸·잘못된 줄은 무시", () => {
		expect(maskText("바보바보야", { hide: "", replace: "바보=멍청이\n바보바보=두번\n=없음\n그냥" })).toBe("두번야");
	});
	it("정규식 기호도 글자 그대로", () => {
		expect(maskText("a.b a*b", { hide: "a.b", replace: "a*b=별" })).toBe("●●● 별");
	});
});

describe("extractRangeText (북마크·텍스트 카드 줄바꿈 보존)", () => {
	it("<br>과 여러 <p>/<div> 블록을 선택할 때 줄바꿈이 보존된다", () => {
		const host = document.createElement("div");
		host.id = "msg-1";
		host.innerHTML = `<div class="md"><p>첫째 줄<br>둘째 줄</p><p>셋째 줄</p><div>넷째 줄</div></div>`;
		document.body.appendChild(host);
		try {
			const range = document.createRange();
			range.selectNodeContents(host);
			expect(extractRangeText(host, range, () => false)).toBe("첫째 줄\n둘째 줄\n셋째 줄\n넷째 줄");
		} finally {
			host.remove();
		}
	});
});
