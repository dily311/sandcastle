import { describe, expect, it } from "vitest";
import type { Message_Serialize as Message } from "$ipc/bindings";
import { lintOf } from "./lint";

const msg = (content: string, lint: unknown): Message => ({ id: "m", content, meta: { lint } }) as unknown as Message;
const l = { original: "원문", content: "고친 글", findings: [{ rule: "unclosed", level: "warn", text: "x", fixed: true }], fixedRules: ["unclosed"], at: 1 };

describe("lintOf", () => {
	it("지금 글과 같은 결과만 보여 준다", () => {
		expect(lintOf(msg("고친 글", l))?.original).toBe("원문");
		expect(lintOf(msg("그 뒤 또 고친 글", l))).toBeNull();
		expect(lintOf(msg("고친 글", undefined))).toBeNull();
		expect(lintOf(msg("고친 글", { content: "고친 글" }))).toBeNull();
	});
});
