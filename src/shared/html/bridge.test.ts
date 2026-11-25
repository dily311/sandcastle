import { describe, expect, it, vi } from "vitest";
import { dispatchAction, parseAction, registerComposer } from "./bridge";

describe("HTML 블록 요청", () => {
	it("허용된 모양만 받는다", () => {
		expect(parseAction({ type: "sc-action", action: "send", text: " 문을 연다 " })).toEqual({ action: "send", text: "문을 연다" });
		expect(parseAction({ type: "sc-action", action: "delete", text: "x" })).toBeNull();
		expect(parseAction({ type: "sc-action", action: "fill", text: "   " })).toBeNull();
		expect(parseAction({ type: "other" })).toBeNull();
		expect(parseAction(null)).toBeNull();
		expect(parseAction({ type: "sc-action", action: "fill", text: "a".repeat(5000) })?.text.length).toBe(4000);
	});

	it("슬래시(triggerSlash)는 커맨드로 시작하는 글만, 더 길게", () => {
		expect(parseAction({ type: "sc-action", action: "slash", text: '/setvar key=choiceText "문 열기"' })).toEqual({ action: "slash", text: '/setvar key=choiceText "문 열기"' });
		expect(parseAction({ type: "sc-action", action: "slash", text: "/run 다음 장면" })?.text).toBe("/run 다음 장면");
		expect(parseAction({ type: "sc-action", action: "slash", text: "/:세트.버튼" })?.text).toBe("/:세트.버튼");
		// 커맨드가 아닌 글은 받지 않는다 (메시지로 보내지면 안 됨)
		expect(parseAction({ type: "sc-action", action: "slash", text: "안녕" })).toBeNull();
		expect(parseAction({ type: "sc-action", action: "slash", text: "// 주석만" })).toBeNull();
		expect(parseAction({ type: "sc-action", action: "slash", text: "/echo " + "a".repeat(30000) })?.text.length).toBe(20000);
	});

	it("마지막에 등록한 쪽이 받고, 닫으면 앞 쪽이, 모두 닫으면 아무도 받지 않는다", () => {
		const a = vi.fn();
		const b = vi.fn();
		const offA = registerComposer(a);
		const offB = registerComposer(b);
		dispatchAction({ action: "fill", text: "x" });
		expect(b).toHaveBeenCalledOnce();
		expect(a).not.toHaveBeenCalled();
		offB(); // 뒤에 연 화면(폰)을 닫으면 앞 화면(대화)이 다시 받는다
		expect(dispatchAction({ action: "fill", text: "y" })).toBe(true);
		expect(a).toHaveBeenCalledWith({ action: "fill", text: "y" });
		offA();
		expect(dispatchAction({ action: "fill", text: "z" })).toBe(false);
	});
});
