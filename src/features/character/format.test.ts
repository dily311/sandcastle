import { describe, expect, it } from "vitest";
import { preview, toBlocks } from "./format";

describe("toBlocks", () => {
	it("줄마다 문단을 만들고 빈 줄은 버린다", () => {
		const blocks = toBlocks("첫 줄\n\n  둘째 줄  ");
		expect(blocks).toEqual([
			{ kind: "p", parts: [{ kind: "text", text: "첫 줄" }] },
			{ kind: "p", parts: [{ kind: "text", text: "둘째 줄" }] },
		]);
	});

	it("*지문*과 \"대사\"를 나눈다", () => {
		const [block] = toBlocks('*웃는다* "안녕" 하고 말했다');
		expect(block).toEqual({
			kind: "p",
			parts: [
				{ kind: "action", text: "웃는다" },
				{ kind: "text", text: " " },
				{ kind: "speech", text: '"안녕"' },
				{ kind: "text", text: " 하고 말했다" },
			],
		});
	});

	it("<cg> 태그는 문단 사이 블록이 된다", () => {
		const blocks = toBlocks("앞<cg> smile </cg>뒤");
		expect(blocks.map((b) => b.kind)).toEqual(["p", "cg", "p"]);
		expect(blocks[1]).toEqual({ kind: "cg", key: "smile" });
	});
});

describe("preview", () => {
	it("태그·기호를 지우고 공백을 합친다", () => {
		expect(preview('*웃음*  "안녕"\n<cg>x</cg>')).toBe("웃음 안녕");
	});

	it("길면 자르고 말줄임표를 붙인다", () => {
		expect(preview("가".repeat(50), 10)).toBe(`${"가".repeat(10)}…`);
	});
});
