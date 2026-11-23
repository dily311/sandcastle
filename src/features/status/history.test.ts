import { describe, expect, it } from "vitest";
import { varsAt } from "./history";

const m = (varChanges?: { key: string; from: string | null; to: string }[]) => ({ meta: varChanges ? { varChanges } : null });

describe("지난 시점의 변수 (5-E-6-1)", () => {
	const messages = [
		m(),
		m([{ key: "hp", from: "100", to: "90" }]),
		m(),
		m([
			{ key: "hp", from: "90", to: "70" },
			{ key: "met", from: null, to: "true" },
		]),
	];
	const now = { hp: "70", met: "true", name: "유나" };

	it("그 메시지 뒤의 AI 바뀜을 되돌린다", () => {
		expect(varsAt(now, messages, 3)).toEqual(now);
		expect(varsAt(now, messages, 2)).toEqual({ hp: "90", name: "유나" });
		expect(varsAt(now, messages, 1)).toEqual({ hp: "90", name: "유나" });
		expect(varsAt(now, messages, 0)).toEqual({ hp: "100", name: "유나" });
	});

	it("지금 값은 건드리지 않는다", () => {
		varsAt(now, messages, 0);
		expect(now.hp).toBe("70");
	});
});
