import { describe, expect, it, vi } from "vitest";

// 5-E-20-6-11: 실패하면 '다시 실행' — 앞 실행의 /input 답은 다시 묻지 않는다
let fail = true;
const asked: string[] = [];
const appended: { content: string }[] = [];
const shown: { tone: string; message: string; action?: { label: string; run: () => void } }[] = [];

vi.mock("$shared/ipc/client", () => ({
	call: async (p: unknown) => {
		if (p instanceof Error) throw p;
		return p;
	},
	commands: {
		slashGenerate: (_c: string, prompt: string) => (fail ? new Error("공급자 오류") : { text: `답: ${prompt}`, truncated: false, prompt: null }),
		appendMessage: (m: { content: string }) => {
			appended.push(m);
			return { ...m, id: `m${appended.length}`, seq: appended.length };
		},
		saveMessagePrompt: () => null,
		slashCancel: () => true,
	},
}));
vi.mock("$shared/stores/confirm.svelte", () => ({
	promptText: async (r: { title: string }) => {
		asked.push(r.title);
		return "숲";
	},
	chooseFrom: vi.fn(),
	confirmAction: vi.fn(),
}));
vi.mock("$shared/stores/toast.svelte", () => ({ toasts: { info: vi.fn(), error: vi.fn(), success: vi.fn(), show: (t: (typeof shown)[number]) => shown.push(t) } }));
vi.mock("$shared/stores/aiTools.svelte", () => ({ pluginSetting: { value: { quickReply: { aiLimit: 5, globals: {} } }, save: vi.fn() } }));
vi.mock("$shared/stores/plugins.svelte", () => ({ plugins: { on: () => false } }));
vi.mock("$features/ghost/ghost.svelte", () => ({ ghost: { cancel: vi.fn(), run: vi.fn() } }));
vi.mock("$features/character/library.svelte", () => ({
	library: {
		conversation: () => ({ id: "c", characterId: "ch", personaId: "p", meta: {} }),
		character: () => ({ name: "세라" }),
		persona: () => ({ name: "민" }),
		personas: [],
		presets: [],
		patchMeta: vi.fn(),
		refresh: vi.fn(),
	},
}));
vi.mock("./quickReply.svelte", () => ({ quickReplies: { loaded: true, load: vi.fn(), sets: [] }, toInput: (s: unknown) => s }));

const { runSlash } = await import("./commands");
const session = { convId: "c", busy: false, messages: [] as unknown[], send: vi.fn(), regenerate: vi.fn(), removeMany: vi.fn(), cancel: vi.fn() };

describe("실패하면 다시 실행 (5-E-20-6-11)", () => {
	it("어디서 실패했는지 알리고, 다시 실행하면 입력을 다시 묻지 않고 이어서 끝까지", async () => {
		await runSlash('/input 주제는? | /gen {{pipe}} 이야기 | /sendas name="{{char}}"', session as never);
		expect(asked).toEqual(["주제는?"]);
		expect(appended).toHaveLength(0);
		const failed = shown.find((t) => t.tone === "error");
		expect(failed?.message).toBe("/gen에서 실패해 멈춤");
		expect(failed?.action?.label).toBe("다시 실행");

		fail = false;
		failed?.action?.run();
		await vi.waitFor(() => expect(appended).toHaveLength(1));
		expect(asked).toEqual(["주제는?"]); // 다시 묻지 않음
		expect(appended[0].content).toBe("답: 숲 이야기");
	});
});
