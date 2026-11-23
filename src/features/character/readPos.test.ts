import { beforeEach, describe, expect, it } from "vitest";
import { getReadPos, MAX_KEPT, setReadPos } from "./readPos";

describe("읽던 자리 (5-E-6-2-3)", () => {
	beforeEach(() => localStorage.clear());

	it("대화마다 메시지와 비율을 기억한다", () => {
		expect(getReadPos("c1")).toBeNull();
		setReadPos("c1", { messageId: "m5", ratio: 0.4 });
		setReadPos("c2", { messageId: "m9", ratio: 0 });
		expect(getReadPos("c1")).toEqual({ messageId: "m5", ratio: 0.4 });
		expect(getReadPos("c2")).toEqual({ messageId: "m9", ratio: 0 });
	});

	it("비율은 0~1로, 망가진 값은 없는 것으로", () => {
		localStorage.setItem("sandcastle.readPos", JSON.stringify({ a: { messageId: "m1", ratio: 7, at: 0 }, b: { ratio: 0.5, at: 0 } }));
		expect(getReadPos("a")).toEqual({ messageId: "m1", ratio: 1 });
		expect(getReadPos("b")).toBeNull();
		localStorage.setItem("sandcastle.readPos", "{망가짐");
		expect(getReadPos("a")).toBeNull();
	});

	it("최근에 읽은 것만 남긴다", () => {
		for (let i = 0; i <= MAX_KEPT; i++) setReadPos(`c${i}`, { messageId: "m", ratio: 0 }, i);
		expect(getReadPos("c0")).toBeNull();
		expect(getReadPos("c1")).not.toBeNull();
		expect(getReadPos(`c${MAX_KEPT}`)).not.toBeNull();
	});
});
