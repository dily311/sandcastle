import { describe, expect, it } from "vitest";
import { planSettings } from "./storyPlan.svelte";

describe("planSettings", () => {
	it("꺼진 게임·망가진 설정은 꺼짐", () => {
		expect(planSettings(null).enabled).toBe(false);
		expect(planSettings("{}").enabled).toBe(false);
		expect(planSettings("not json").enabled).toBe(false);
		expect(planSettings({ storyPlan: { enabled: "yes" } }).enabled).toBe(false);
	});
	it("문자열·객체 둘 다 읽고 턴 수는 3~100으로", () => {
		expect(planSettings({ storyPlan: { enabled: true } })).toEqual({ enabled: true, everyNTurns: 10 });
		expect(planSettings(JSON.stringify({ storyPlan: { enabled: true, everyNTurns: 1 } })).everyNTurns).toBe(3);
		expect(planSettings({ storyPlan: { enabled: true, everyNTurns: 500 } }).everyNTurns).toBe(100);
	});
});
