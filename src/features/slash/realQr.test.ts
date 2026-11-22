import { beforeEach, describe, expect, it, vi } from "vitest";

// 5-E-20-6-5: 사용자가 준 실제 SillyTavern QR 4개(src-tauri/tests/sillyTavern/qr)를 앱 연결부 그대로(가짜 AI·가짜 저장소) 끝까지 돌린다.
const gens: string[] = [];
const appended: { role: string; name: string | null; content: string; meta: unknown }[] = [];
const echoed: string[] = [];
let answer: string | null = "숲의 축제";

vi.mock("$shared/ipc/client", () => ({
	call: async (p: unknown) => p,
	commands: {
		slashGenerate: (_c: string, prompt: string) => {
			gens.push(prompt);
			return { text: `AI가 쓴 글 ${gens.length}`, truncated: false };
		},
		appendMessage: (m: { role: string; name: string | null; content: string; meta: unknown }) => {
			appended.push(m);
			return { ...m, id: `m${appended.length}`, seq: appended.length };
		},
	},
}));
vi.mock("$shared/stores/confirm.svelte", () => ({ promptText: async () => answer, chooseFrom: vi.fn(), confirmAction: vi.fn() }));
vi.mock("$shared/stores/toast.svelte", () => ({
	toasts: { info: vi.fn(), error: vi.fn(), success: vi.fn(), show: (t: { message: string }) => echoed.push(t.message) },
}));
vi.mock("$shared/stores/aiTools.svelte", () => ({ pluginSetting: { value: { quickReply: { aiLimit: 5, globals: {} } }, save: vi.fn() } }));
vi.mock("$shared/stores/plugins.svelte", () => ({ plugins: { on: () => false } }));
vi.mock("$features/character/library.svelte", () => ({
	library: {
		conversation: () => ({ id: "c", characterId: "ch", personaId: "p", meta: {} }),
		character: () => ({ name: "세라피나" }),
		persona: () => ({ name: "민" }),
		personas: [],
		presets: [],
		patchMeta: vi.fn(),
		refresh: vi.fn(),
	},
}));
vi.mock("./quickReply.svelte", () => ({ quickReplies: { loaded: true, load: vi.fn(), sets: [] }, toInput: (s: unknown) => s }));

const { slashEnv } = await import("./commands");
const { runScript } = await import("./script");

const QR = import.meta.glob("../../../src-tauri/tests/sillyTavern/qr/*.json", { eager: true, import: "default" }) as Record<string, { message: string; label: string }>;
const qr = (name: string) => QR[`../../../src-tauri/tests/sillyTavern/qr/${name}`].message;
const run = async (name: string) => {
	const session = { convId: "c", busy: false, messages: [] as unknown[], send: vi.fn(), regenerate: vi.fn(), removeMany: vi.fn() };
	const env = slashEnv(session as never);
	const r = await runScript(qr(name), env);
	await env.flush();
	return r;
};

beforeEach(() => {
	gens.length = 0;
	appended.length = 0;
	echoed.length = 0;
	answer = "숲의 축제";
});

describe("실제 QR 실행 (5-E-20-6-5)", () => {
	it("분석: 입력 → AI 분석 → AI에게 숨긴 접기 메시지로", async () => {
		const r = await run("(분석)텍스트만.qr.json");
		expect(r.aborted).toBe(false);
		expect(gens).toHaveLength(1);
		expect(gens[0]).toContain("The main topic: [숲의 축제]");
		expect(gens[0]).toContain("세라피나"); // {{char}}
		expect(appended).toHaveLength(1);
		expect(appended[0].role).toBe("system");
		expect(appended[0].meta).toEqual({ hidden: true, comment: true });
		expect(appended[0].content).toContain("(Q. 숲의 축제)에 따른 분석결과");
		expect(appended[0].content).toContain("AI가 쓴 글 1");
	});
	it("분석: 입력을 비우면 알리고 멈춤 (AI를 부르지 않음)", async () => {
		answer = null;
		const r = await run("(분석)텍스트만.qr.json");
		expect(r.aborted).toBe(true);
		expect(gens).toHaveLength(0);
		expect(echoed).toEqual(["생성을 취소하였습니다."]);
	});
	it("자동사냥: 긴 지시(본문 중간 따옴표 포함)가 끝까지 AI로 가고 캐릭터 말로", async () => {
		await run("[생성] ❤️_🔥 자동사냥.qr.json");
		expect(gens).toHaveLength(1);
		expect(gens[0]).toContain('User Input: "숲의 축제"');
		expect(gens[0].trimEnd().endsWith("</System>")).toBe(true);
		expect(appended).toEqual([expect.objectContaining({ role: "assistant", name: null, content: "AI가 쓴 글 1" })]);
	});
	it("무물: 질문이 {{pipe}}로 지시 안에 두 번 들어가고 캐릭터 말로", async () => {
		await run("무물🎤.qr.json");
		expect(gens[0].split('"숲의 축제"').length - 1).toBe(2);
		expect(appended).toEqual([expect.objectContaining({ role: "assistant", content: "AI가 쓴 글 1" })]);
	});
	it("뉴스: 변수에 지시를 넣고 AI → {{groupNotMuted}}(캐릭터) 말로 → 변수 지움", async () => {
		await run("📺뉴스.qr.json");
		expect(gens).toHaveLength(1);
		expect(gens[0].startsWith("Pause the story. Generate 3 news articles.")).toBe(true);
		// SillyTavern과 같게 본문의 이스케이프 안 된 | 에서 잘린다
		expect(gens[0].trimEnd().endsWith("title: [Title]")).toBe(true);
		expect(appended).toEqual([expect.objectContaining({ role: "assistant", name: null, content: "AI가 쓴 글 1" })]);
	});
});
