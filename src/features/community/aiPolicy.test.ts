import { describe, expect, it } from "vitest";
import { decide, describeCall } from "./aiPolicy";

const o = (p: Partial<Parameters<typeof decide>[0]> = {}) => ({ fromPanel: false, inCommand: false, inHook: false, aiAuto: false, ...p });

describe("AI 호출 실행 주체", () => {
	it("누른 명령 안(메인 프레임)은 사용자, 확인 없음", () => {
		expect(decide(o({ inCommand: true }))).toEqual({ by: "user", ask: false });
		expect(decide(o({ inCommand: true, aiAuto: true }))).toEqual({ by: "user", ask: false });
	});

	it("훅 안은 항상 auto — 스위치가 없으면 백엔드가 거절", () => {
		expect(decide(o({ inHook: true }))).toEqual({ by: "auto", ask: false });
		expect(decide(o({ inHook: true, inCommand: true }))).toEqual({ by: "auto", ask: false });
	});

	it("패널은 명령 중이어도 사용자 것으로 치지 않는다: 스위치가 켜졌으면 auto, 아니면 확인", () => {
		expect(decide(o({ fromPanel: true, inCommand: true }))).toEqual({ by: "user", ask: true });
		expect(decide(o({ fromPanel: true, aiAuto: true }))).toEqual({ by: "auto", ask: false });
	});

	it("아무 계기 없이 부르면 확인을 받는다", () => {
		expect(decide(o())).toEqual({ by: "user", ask: true });
		expect(decide(o({ aiAuto: true }))).toEqual({ by: "auto", ask: false });
	});

	it("확인 창: 긴 글은 잘라 보이고 출력 토큰은 상한 안으로", () => {
		const d = describeCall("요약기", { prompt: "가".repeat(500), maxTokens: 99_999 });
		expect(d.title).toContain("요약기");
		expect(d.message).toContain("최대 출력 2000 토큰");
		expect(d.message).toContain("…");
		expect(d.message.length).toBeLessThan(500);
		expect(describeCall("x", null).message).toContain("(비어 있음)");
		expect(describeCall("x", { prompt: "a" }).message).toContain("최대 출력 500 토큰");
	});
});
