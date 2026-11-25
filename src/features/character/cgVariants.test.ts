import { describe, expect, it } from "vitest";
import { baseKey, cgVariants, pickVariants } from "./cgVariants";

describe("CG 같은 키 묶기 (5-E-25)", () => {
	it("끝의 -숫자만 뗀다", () => {
		expect(baseKey("smile-2")).toBe("smile");
		expect(baseKey("blue-eyes")).toBe("blue-eyes");
		expect(baseKey("-1")).toBe("-1");
	});
	it("묶음 키로 모으고, 메시지마다 고정된 하나", () => {
		const v = cgVariants(
			[
				{ key: "smile", src: "a" },
				{ key: "smile-1", src: "b" },
				{ key: "smile-2", src: "c" },
				{ key: "cry", src: "d" },
			],
			true,
		);
		expect(v.smile).toEqual(["a", "b", "c"]);
		expect(v["smile-1"]).toEqual(["b"]);
		const p1 = pickVariants(v, "m1").smile;
		expect(pickVariants(v, "m1").smile).toBe(p1);
		expect(new Set(["m1", "m2", "m3", "m4", "m5", "m6", "m7", "m8"].map((m) => pickVariants(v, m).smile)).size).toBeGreaterThan(1);
		expect(cgVariants([{ key: "smile-1", src: "b" }], false).smile).toBeUndefined();
	});
});
