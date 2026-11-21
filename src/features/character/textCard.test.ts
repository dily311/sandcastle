import { describe, expect, it } from "vitest";
import { maskText } from "./textCard";

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
