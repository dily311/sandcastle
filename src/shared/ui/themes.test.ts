import { describe, expect, it } from "vitest";
import { legacyColors, withLegacy } from "./themes";

// 예전 테마 JSON의 cssContent 모양 (옵시디언식 변수, body.theme-light / body.theme-dark)
const OLD = `
body.theme-light {
  --color-00: #eff1f5; /* Base */
  --color-80: #4c4f69;
  --background-primary: var(--color-00);
  --sidebar-bg: #e6e9ef;
  --text-color: var(--color-80);
  --accent-color: #1e66f5;
  --border-color: var(--missing);
}
body.theme-dark {
  --background-primary: #1e1e2e;
  --text-color: #cdd6f4;
}`;

describe("예전 테마 → 새 테마 변수", () => {
	it("모드별로 변수를 옮기고 var()를 풀어 쓴다", () => {
		const c = legacyColors(OLD);
		expect(c.light["--surface-2"]).toBe("#eff1f5");
		expect(c.light["--bg"]).toBe("#e6e9ef");
		expect(c.light["--text"]).toBe("#4c4f69");
		expect(c.light["--accent"]).toBe("#1e66f5");
		// 풀 수 없는 var()는 넣지 않는다
		expect(c.light["--border"]).toBeUndefined();
		expect(c.dark["--surface-2"]).toBe("#1e1e2e");
		expect(c.dark["--text"]).toBe("#cdd6f4");
	});

	it("body 블록은 두 모드 모두의 기본, 테마 colors가 이긴다", () => {
		const c = withLegacy({ light: { "--text": "#000000" }, dark: {} }, "body { --text-color: #111111; --background-primary: #fafafa; }");
		expect(c.light["--text"]).toBe("#000000");
		expect(c.dark["--text"]).toBe("#111111");
		expect(c.dark["--surface-2"]).toBe("#fafafa");
	});

	it("새 형식 CSS만 있으면 아무것도 바꾸지 않는다", () => {
		expect(legacyColors(":root { --bg: red; }")).toEqual({ light: {}, dark: {} });
	});
});
