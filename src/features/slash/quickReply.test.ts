import { describe, expect, it, vi } from "vitest";

vi.mock("$shared/stores/aiTools.svelte", () => ({
	pluginSetting: {
		value: {
			quickReply: {
				sets: [
					{ id: "a", name: "지시", enabled: true, autoSend: false, items: [{ id: "1", label: "넣기", message: "안녕", hidden: false }] },
					{ id: "b", name: "전투", enabled: false, autoSend: true, items: [] },
				],
			},
		},
	},
}));
vi.mock("$features/character/library.svelte", () => ({ library: {} }));
vi.mock("./commands", () => ({ isSlash: (t: string) => t.startsWith("/"), runSlash: vi.fn() }));

const { chosenIds, runQuickReply, setsFor } = await import("./quickReply");
const conv = (meta: unknown) => ({ id: "c", meta }) as never;

describe("빠른 답장 세트 고르기", () => {
	it("대화에서 고르지 않으면 켜 둔 세트, 고르면 그 세트만", () => {
		expect(setsFor(conv(null)).map((s) => s.id)).toEqual(["a"]);
		expect(setsFor(conv({ quickReplySets: ["b"] })).map((s) => s.id)).toEqual(["b"]);
		expect(setsFor(conv({ quickReplySets: [] }))).toEqual([]);
		expect(chosenIds(conv({ quickReplySets: "x" }))).toBeNull();
	});
	it("바로 보내기가 꺼진 세트의 글은 입력칸에 덧붙이기만 (보내지 않음)", async () => {
		let draft = "처음";
		const send = vi.fn();
		const session = { busy: false, send } as never;
		const set = setsFor(conv(null))[0];
		await runQuickReply(set.items[0], set, session, { get: () => draft, set: (v) => (draft = v) });
		expect(draft).toBe("처음\n안녕");
		expect(send).not.toHaveBeenCalled();
	});
});
