import { describe, expect, it } from "vitest";
import type { RuntimePlugin } from "$ipc/bindings";
import { chain } from "./postprocess";

const plugin = (id: string, mode: string, granted = ["messages.process"]): RuntimePlugin => ({ id, name: id, version: "1", hostUrl: "", granted, processMode: mode, panels: [], commands: [] });
const msg = { id: "m1", role: "assistant", content: "Hello \"x", conversationId: "v1", name: null };

describe("후처리 차례", () => {
	it("자동 고침만 글을 바꾸고 다음 플러그인에 넘긴다", async () => {
		const seen: string[] = [];
		const emit = async (id: string, _e: string, data: unknown) => {
			const c = (data as { message: { content: string } }).message.content;
			seen.push(`${id}:${c}`);
			if (id === "a") return { content: c + "\"", notes: ["따옴표 닫음"] };
			if (id === "b") return { content: "무시됨", notes: [{ level: "warn", text: "표시만" }] };
			return { content: c + "!" };
		};
		const r = await chain(msg, [plugin("a", "fix"), plugin("b", "notes"), plugin("c", "fix")], emit);
		expect(r.content).toBe('Hello "x"!');
		expect(seen).toEqual(['a:Hello "x', 'b:Hello "x"', 'c:Hello "x"']);
		expect(r.results).toEqual([
			{ pluginId: "a", changed: true, notes: [{ level: "info", text: "따옴표 닫음" }] },
			{ pluginId: "b", changed: false, notes: [{ level: "warn", text: "표시만" }] },
			{ pluginId: "c", changed: true, notes: [] },
		]);
	});

	it("권한 없는 플러그인은 부르지 않고, 실패한 플러그인은 건너뛴다", async () => {
		const called: string[] = [];
		const emit = async (id: string) => {
			called.push(id);
			if (id === "bad") throw new Error("시간 초과");
			return null;
		};
		const r = await chain(msg, [plugin("none", "fix", []), plugin("bad", "fix"), plugin("quiet", "fix")], emit);
		expect(called).toEqual(["bad", "quiet"]);
		expect(r.content).toBe(msg.content);
		expect(r.results).toEqual([]);
		expect(r.errors).toEqual([{ pluginId: "bad", message: "시간 초과" }]);
	});
});
