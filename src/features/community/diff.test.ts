import { describe, expect, it } from "vitest";
import { diffLines, fold, show } from "./diff";

describe("줄 차이", () => {
	it("바뀐 줄만 지움·더함으로", () => {
		expect(diffLines("a\nb\nc", "a\nB\nc")).toEqual([
			{ kind: "same", text: "a" },
			{ kind: "del", text: "b" },
			{ kind: "add", text: "B" },
			{ kind: "same", text: "c" },
		]);
	});
	it("빈 글", () => {
		expect(diffLines("", "x")).toEqual([{ kind: "add", text: "x" }]);
		expect(diffLines("x", "")).toEqual([{ kind: "del", text: "x" }]);
		expect(diffLines("", "")).toEqual([]);
	});
	it("값 보여주기", () => {
		expect(show(["a", "b"])).toBe("a, b");
		expect(show(false)).toBe("꺼짐");
		expect(show(null)).toBe("");
	});
	it("같은 줄은 접고 바뀐 곳 앞뒤만", () => {
		const a = Array.from({ length: 10 }, (_, i) => `l${i}`).join("\n");
		const b = a + "\nnew";
		const f = fold(diffLines(a, b), 2);
		expect(f[0]).toEqual({ kind: "skip", count: 8 });
		expect(f.slice(1)).toEqual([
			{ kind: "same", text: "l8" },
			{ kind: "same", text: "l9" },
			{ kind: "add", text: "new" },
		]);
	});
});
