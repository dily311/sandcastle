// 테마 CSS 클래스 문서(docs/테마 CSS 클래스.md)에 적은 `sc-` 클래스가 코드에 실제로 있는지 (5-E-17-2)
import { describe, expect, it } from "vitest";

const doc = Object.values(import.meta.glob("/docs/테마 CSS 클래스.md", { query: "?raw", import: "default", eager: true }))[0] as string;
const code = Object.values(import.meta.glob(["/src/**/*.svelte", "/src/**/*.ts", "!/src/**/*.test.ts"], { query: "?raw", import: "default", eager: true })).join("\n") as string;

describe("테마 CSS 클래스", () => {
	const names = [...new Set([...(doc ?? "").matchAll(/`(sc-[a-z-]+)`/g)].map((m) => m[1]))];
	it("문서를 읽음", () => {
		expect(names.length).toBeGreaterThan(20);
	});
	it.each(names)("%s 가 코드에 있음", (name) => {
		expect(code).toMatch(new RegExp(`[\\s"'.:]${name}(?![a-z-])`));
	});
});
