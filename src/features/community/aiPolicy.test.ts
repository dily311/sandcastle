import { describe, expect, it } from "vitest";
import { decide, describeCall } from "./aiPolicy";

const o = (p: Partial<Parameters<typeof decide>[0]> = {}) => ({ fromPanel: false, commandAllowance: false, inHook: false, aiAuto: false, ...p });

describe("AI 호출 실행 주체", () => {
	it("누른 명령의 몫이 남아 있으면(메인 프레임) 사용자, 확인 없음", () => {
		expect(decide(o({ commandAllowance: true }))).toEqual({ by: "user", ask: false });
		expect(decide(o({ commandAllowance: true, aiAuto: true }))).toEqual({ by: "user", ask: false });
	});

	it("몫이 없으면(다 썼거나 시간이 지남) 명령 안이어도 사용자 것이 아니다", () => {
		expect(decide(o({ commandAllowance: false }))).toEqual({ by: "auto", ask: false });
	});

	it("훅 안은 항상 auto — 스위치가 없으면 백엔드가 거절", () => {
		expect(decide(o({ inHook: true }))).toEqual({ by: "auto", ask: false });
		expect(decide(o({ inHook: true, commandAllowance: true }))).toEqual({ by: "auto", ask: false });
	});

	it("패널은 명령 몫을 못 쓴다: 스위치가 켜졌으면 auto, 아니면 확인", () => {
		expect(decide(o({ fromPanel: true, commandAllowance: true }))).toEqual({ by: "user", ask: true });
		expect(decide(o({ fromPanel: true, aiAuto: true }))).toEqual({ by: "auto", ask: false });
	});

	it("눈에 안 보이는 프레임의 호출은 확인 창을 띄우지 못한다 (auto → 스위치 없으면 거절)", () => {
		expect(decide(o())).toEqual({ by: "auto", ask: false });
		expect(decide(o({ aiAuto: true }))).toEqual({ by: "auto", ask: false });
	});

	it("확인 창: 플러그인 이름과 id, 글자 수, 시스템 지시까지 보이고 긴 글은 잘리며 출력 토큰은 상한 안으로", () => {
		const d = describeCall("요약기", "summarizer", { prompt: "가".repeat(500), system: "나".repeat(300), maxTokens: 99_999 });
		expect(d.title).toContain("요약기");
		expect(d.title).toContain("summarizer");
		expect(d.message).toContain("800자");
		expect(d.message).toContain("최대 출력 2000 토큰");
		expect(d.message).toContain("시스템 지시:");
		expect(d.message).toContain("…");
		expect(describeCall("x", "x", null).message).toContain("(비어 있음)");
		expect(describeCall("x", "x", { prompt: "a" }).message).toContain("최대 출력 500 토큰");
	});
});
