import { describe, expect, it } from "vitest";
import type { VarDef } from "$features/character/library.svelte";
import { coerceVar, varRows } from "./vars";

const def = (key: string, p: Partial<VarDef> = {}): VarDef => ({ key, description: "", kind: "string", defaultValue: null, min: null, max: null, maxStep: null, aiRead: false, aiWrite: false, ...p });

describe("변수 정의 쓰기 (5-F-4-3)", () => {
	it("정의된 변수가 정의 순서대로 먼저(값이 없어도), 정의 없는 것은 그 뒤에 이름순", () => {
		const rows = varRows({ z: "1", hp: "7", a: "x" }, [def("mood"), def("hp", { kind: "number" })]);
		expect(rows.map((r) => [r.name, r.value, !!r.def])).toEqual([
			["mood", undefined, true],
			["hp", "7", true],
			["a", "x", false],
			["z", "1", false],
		]);
	});

	it("수는 최소·최대 안으로, 수가 아니면 거절. 참거짓은 true/false만. 정의 없으면 그대로", () => {
		const hp = def("hp", { kind: "number", min: 0, max: 100 });
		expect(coerceVar(hp, " 150 ")).toEqual({ value: "100" });
		expect(coerceVar(hp, "-3")).toEqual({ value: "0" });
		expect(coerceVar(hp, "7.5")).toEqual({ value: "7.5" });
		expect(coerceVar(hp, "많음")).toHaveProperty("error");
		expect(coerceVar(hp, "")).toHaveProperty("error");
		const met = def("met", { kind: "boolean" });
		expect(coerceVar(met, "TRUE")).toEqual({ value: "true" });
		expect(coerceVar(met, "yes")).toHaveProperty("error");
		expect(coerceVar(undefined, "아무 글")).toEqual({ value: "아무 글" });
	});
});
