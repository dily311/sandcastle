import { describe, expect, it } from "vitest";
import { highestRisk, permissionInfo, sortPermissions } from "./permissions";

// 백엔드가 받는 권한 전부 (service/community/manifest.rs)
const KNOWN = [
	"messages.process",
	"prompt.modify",
	"ui.panel",
	"ui.command",
	"ai",
	"storage",
	...["character", "persona", "lorebook", "chat"].flatMap((k) => [`data.read:${k}`, `data.write:${k}`]),
];

describe("권한 설명", () => {
	it("백엔드가 받는 권한은 모두 설명이 있다", () => {
		for (const p of KNOWN) {
			const i = permissionInfo(p);
			expect(i.label, p).not.toBe(p);
			expect(i.desc.length, p).toBeGreaterThan(10);
		}
	});

	it("모르는 권한은 가장 위험한 것으로", () => {
		expect(permissionInfo("network").risk).toBe("high");
		expect(permissionInfo("data.read:secrets").risk).toBe("high");
	});

	it("대화 읽기와 AI 호출은 높음, 화면은 낮음", () => {
		expect(permissionInfo("ai").risk).toBe("high");
		expect(permissionInfo("data.read:chat").risk).toBe("high");
		expect(permissionInfo("data.read:character").risk).toBe("mid");
		expect(permissionInfo("ui.panel").risk).toBe("low");
	});

	it("위험한 것부터 늘어놓고 가장 높은 위험을 알려 준다", () => {
		expect(sortPermissions(["ui.panel", "storage", "data.write:persona", "ai"])).toEqual(["ai", "data.write:persona", "storage", "ui.panel"]);
		expect(highestRisk(["ui.panel", "ui.command"])).toBe("low");
		expect(highestRisk(["ui.panel", "ai"])).toBe("high");
		expect(highestRisk([])).toBeNull();
	});
});
