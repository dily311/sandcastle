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

	it("마지막에 등록한 쪽이 받고, 해제하면 아무도 받지 않는다", () => {
		const a = vi.fn();
		const b = vi.fn();
		const offA = registerComposer(a);
		const offB = registerComposer(b);
		dispatchAction({ action: "fill", text: "x" });
		expect(b).toHaveBeenCalledOnce();
		expect(a).not.toHaveBeenCalled();
		offA(); // 이미 바뀐 등록은 건드리지 않음
		expect(dispatchAction({ action: "fill", text: "y" })).toBe(true);
		offB();
		expect(dispatchAction({ action: "fill", text: "z" })).toBe(false);
	});
});
