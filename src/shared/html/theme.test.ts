import { describe, expect, it } from "vitest";
import { themeCss } from "./theme";

describe("themeCss", () => {
	it("변수를 :root에 옮기고 바탕·글자색을 정한다", () => {
		const css = themeCss({ "--bg": "#111", "--accent": "rgb(1 2 3)" }, true);
		expect(css).toContain(":root{--bg:#111;--accent:rgb(1 2 3);color-scheme:dark}");
		expect(css).toContain("background:var(--surface)");
	});

	it("빈 값이나 블록을 깨는 값은 뺀다", () => {
		const css = themeCss({ "--bg": "", "--text": "red}body{x:1", "--accent": "#fff" }, false);
		expect(css).toContain(":root{--accent:#fff;color-scheme:light}");
	});
});
