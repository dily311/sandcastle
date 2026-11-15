import { describe, expect, it } from "vitest";
import type { GameDefinition, RenderState } from "$features/game/types";
import { formatGameTime, getConnectionRequirement, normalizeCueType } from "./playUtils";

describe("formatGameTime", () => {
	it("초를 하루 기준 HH:MM으로 바꾼다", () => {
		expect(formatGameTime(0)).toBe("00:00");
		expect(formatGameTime(9 * 3600 + 5 * 60 + 59)).toBe("09:05");
		expect(formatGameTime(25 * 3600)).toBe("01:00");
	});
});

describe("normalizeCueType", () => {
	it("camelCase를 snake_case로 바꾼다", () => {
		expect(normalizeCueType("fadeIn")).toBe("fade_in");
		expect(normalizeCueType(undefined)).toBe("");
	});
});

describe("getConnectionRequirement", () => {
	const state = { flags: { courage: 3 }, inventory: { key1: 1 }, relationships: {} } as unknown as RenderState;
	const view = { items: [{ id: "key1", name: "열쇠" }] } as unknown as GameDefinition;

	it("조건이 없으면 null", () => {
		expect(getConnectionRequirement("", state, view)).toBeNull();
		expect(getConnectionRequirement(null, state, view)).toBeNull();
	});

	it("읽지 못한 조건은 충족되지 않은 것으로 본다", () => {
		expect(getConnectionRequirement("{oops", state, view)).toEqual({ isMet: false, label: "조건 확인 필요" });
	});

	it("flag_gte를 판정한다", () => {
		expect(getConnectionRequirement(JSON.stringify({ type: "flag_gte", key: "courage", value: 3 }), state, view)?.isMet).toBe(true);
		expect(getConnectionRequirement(JSON.stringify({ type: "flagGte", key: "courage", value: 4 }), state, view)?.isMet).toBe(false);
	});

	it("has_item은 아이템 이름으로 라벨을 만든다", () => {
		const r = getConnectionRequirement(JSON.stringify({ type: "has_item", itemId: "key1" }), state, view);
		expect(r).toEqual({ isMet: true, label: "열쇠 1개 보유" });
	});
});
