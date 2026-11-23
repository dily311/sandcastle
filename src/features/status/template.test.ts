import { describe, expect, it } from "vitest";
import { fillTemplate } from "./template";

describe("상태창 틀 (5-F-4-5)", () => {
	it("변수·전역 변수·처음 값·이름을 넣고, 모르는 매크로는 그대로", () => {
		const out = fillTemplate("<b>{{char}}</b> HP {{getvar::hp}} / 기분 {{ GETVAR:: mood }} / 날씨 {{getglobalvar::weather}} / {{user}} {{unknown}} [{{getvar::none}}]", {
			vars: { hp: "70" },
			globals: { weather: "비" },
			defs: [{ key: "mood", description: "", kind: "string", defaultValue: "평온", min: null, max: null, maxStep: null, aiRead: false, aiWrite: false }],
			char: "Aria",
			user: "민",
		});
		expect(out).toBe("<b>Aria</b> HP 70 / 기분 평온 / 날씨 비 / 민 {{unknown}} []");
	});

	it("값은 HTML로 해석되지 않게 이스케이프 (틀의 HTML만 HTML)", () => {
		expect(fillTemplate("<p>{{getvar::note}}</p>", { vars: { note: '<img src=x onerror="alert(1)">' }, globals: {} })).toBe(
			"<p>&lt;img src=x onerror=&quot;alert(1)&quot;&gt;</p>",
		);
	});
});
