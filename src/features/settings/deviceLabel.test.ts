import { describe, expect, it } from "vitest";
import { deviceLabel } from "./deviceLabel";

describe("기기 이름", () => {
	const peers = [
		{ deviceId: "a81d00c4-1e2f", name: "거실 PC" },
		{ deviceId: "c2b4d6f8-0a1b", name: null },
	];
	it("이름이 있으면 이름, 없으면 id 앞 8자리", () => {
		expect(deviceLabel("a81d00c4-1e2f", peers)).toBe("거실 PC");
		expect(deviceLabel("c2b4d6f8-0a1b", peers)).toBe("c2b4d6f8");
		expect(deviceLabel("ffffffff-9999", peers)).toBe("ffffffff");
	});
	it("묶음에 실린 이름이 먼저", () => {
		expect(deviceLabel("c2b4d6f8-0a1b", peers, "작업 노트북")).toBe("작업 노트북");
		expect(deviceLabel("a81d00c4-1e2f", peers, "  ")).toBe("거실 PC");
	});
});
