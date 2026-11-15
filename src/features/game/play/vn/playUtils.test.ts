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

import { LineKind, type StageLine } from "$features/game/types";
import { directionParticle, foldDirectionLines, itemGainLines, objectParticle } from "./playUtils";

const text = (t: string, cues: unknown[] = []) => ({ kind: LineKind.Narration, speaker: null, text: t, cues }) as unknown as StageLine;
const cueOnly = (cue: string) => ({ kind: LineKind.Direction, speaker: null, text: "", cues: [{ type: cue }] }) as unknown as StageLine;

describe("foldDirectionLines", () => {
	it("연출만 있는 줄은 다음 대사 줄에 합친다", () => {
		const out = foldDirectionLines([cueOnly("bg"), text("도착"), cueOnly("bgm"), text("둘째")]);
		expect(out.map((l) => l.text)).toEqual(["도착", "둘째"]);
		expect(out[0].cues).toEqual([{ type: "bg" }]);
		expect(out[1].cues).toEqual([{ type: "bgm" }]);
	});

	it("뒤에 대사가 없으면 마지막 대사 줄에 붙인다 (이동 문구가 한 줄로 보임)", () => {
		const out = foldDirectionLines([text("숲 입구에 도착"), cueOnly("standingOff"), cueOnly("bg")]);
		expect(out).toHaveLength(1);
		expect(out[0].cues).toEqual([{ type: "standingOff" }, { type: "bg" }]);
	});

	it("대사 줄이 없으면 빈 목록", () => {
		expect(foldDirectionLines([cueOnly("bg")])).toEqual([]);
	});
});

describe("한국어 조사", () => {
	it("을/를, 로/으로", () => {
		expect(objectParticle("열쇠")).toBe("를");
		expect(objectParticle("약초 꾸러미")).toBe("를");
		expect(objectParticle("검")).toBe("을");
		expect(directionParticle("오두막")).toBe("으로");
		expect(directionParticle("샘터")).toBe("로");
		expect(directionParticle("마을")).toBe("로");
	});
});

describe("itemGainLines", () => {
	it("얻은 아이템을 대사 줄로 만든다", () => {
		const events = [{ type: "itemAdded", itemId: "a", quantity: 1 }, { type: "itemAdded", itemId: "b", quantity: 3 }] as never;
		const lines = itemGainLines(events, [{ id: "a", name: "검" }, { id: "b", name: "약초" }]);
		expect(lines.map((l) => l.text)).toEqual(["검을 얻었다.", "약초 3개를 얻었다."]);
	});
});
