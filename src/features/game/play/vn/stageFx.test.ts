import { describe, expect, it } from "vitest";
import { durationMs, stageFx } from "./stageFx.svelte";

describe("stageFx (5-E-22·23)", () => {
	it("이름:값의 시간", () => {
		expect(durationMs("2s", 500)).toBe(2000);
		expect(durationMs("300ms", 500)).toBe(300);
		expect(durationMs("1.5", 500)).toBe(1500);
		expect(durationMs("빠르게", 500)).toBe(500);
		expect(durationMs("99s", 500)).toBe(10_000);
	});
	it("계속 효과는 clear까지, 한 번짜리는 key가 오른다, 모르는 이름·모션은 무시", () => {
		stageFx.reset();
		stageFx.run([{ type: "effect", name: "snow" }, { type: "effect", name: "blackout", param: "2s" }, { type: "effect", name: "boom" }]);
		expect(stageFx.ambient).toBe("snow");
		expect(stageFx.shot.blackout?.ms).toBe(2000);
		const k = stageFx.shot.blackout!.key;
		stageFx.run([{ type: "effect", name: "blackout" }, { type: "motion", characterId: "yuna", name: "jump" }, { type: "motion", characterId: "yuna", name: "fly" }]);
		expect(stageFx.shot.blackout!.key).toBeGreaterThan(k);
		expect(stageFx.motions.yuna.name).toBe("jump");
		stageFx.run([{ type: "effect", name: "clear" }]);
		expect(stageFx.ambient).toBeNull();
	});
});
