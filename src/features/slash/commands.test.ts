import { beforeEach, describe, expect, it, vi } from "vitest";

// 앱 연결부(5-E-20-6-2) 시험: 가짜 IPC·저장소
const call = vi.fn(async (v: unknown) => v);
const patchMeta = vi.fn(async () => {});
const saveSetting = vi.fn(async () => {});
const conv = { id: "c", characterId: "ch", personaId: "p", meta: { variables: { hp: "10", list: '["a"]' } } };
const setting = { value: { quickReply: { aiLimit: 2, globals: { g: "1" } } } as Record<string, unknown>, save: saveSetting };

vi.mock("$shared/ipc/client", () => ({ call: (p: unknown) => call(p), commands: { slashGenerate: (_c: string, t: string) => `AI:${t}`, appendMessage: (m: unknown) => m } }));
vi.mock("$shared/stores/confirm.svelte", () => ({ promptText: vi.fn() }));
vi.mock("$shared/stores/toast.svelte", () => ({ toasts: { info: vi.fn(), error: vi.fn(), success: vi.fn() } }));
vi.mock("$shared/stores/aiTools.svelte", () => ({ pluginSetting: setting }));
vi.mock("$features/character/library.svelte", () => ({
	library: { conversation: () => conv, character: () => ({ name: "세라" }), persona: () => ({ name: "민" }), patchMeta, refresh: vi.fn() },
}));
vi.mock("./quickReply.svelte", () => ({
	quickReplies: { loaded: true, load: vi.fn(), sets: [{ name: "지.시", items: [{ label: "인사", message: "/echo 안녕" }] }] },
}));

const { quickReplySource, slashEnv } = await import("./commands");
const { runScript } = await import("./script");
const session = { convId: "c", busy: false, messages: [], send: vi.fn(), regenerate: vi.fn() } as never;

beforeEach(() => {
	call.mockClear();
	patchMeta.mockClear();
	saveSetting.mockClear();
});

describe("슬래시 커맨드 앱 연결", () => {
	it("AI 호출이 설정의 상한을 넘으면 오류로 멈춘다 (넘은 커맨드는 부르지 않음)", async () => {
		const env = slashEnv(session);
		await expect(runScript("/gen 하나 | /gen 둘 | /gen 셋", env)).rejects.toThrow("상한(2번)을 넘어 /gen");
		expect(call).toHaveBeenCalledTimes(2);
	});
	it("반복 안에서 대화 변수를 여러 번 바꿔도 저장은 끝날 때 한 번", async () => {
		const env = slashEnv(session);
		const r = await runScript("/times 3 {: /incvar hp :} | /addvar key=list z | /getvar key=list index=1", env);
		expect(r.pipe).toBe("z");
		expect(patchMeta).not.toHaveBeenCalled();
		await env.flush();
		expect(patchMeta).toHaveBeenCalledTimes(1);
		expect(patchMeta).toHaveBeenCalledWith("c", { variables: { hp: "13", list: '["a","z"]' } });
	});
	it("/if의 left가 변수 이름이면 대화 변수 값으로 본다, 전역 변수는 설정에 저장", async () => {
		const env = slashEnv(session);
		// /return은 스크립트 전체를 끝낸다 (SillyTavern과 같음) — hp(10) > 5라 "높음"
		expect((await runScript("/if left=hp right=5 rule=gt {: /return 높음 :} | /echo 여기는 안 옴", env)).pipe).toBe("높음");
		const r = await runScript("/setglobalvar key=g 2 | /getglobalvar g", env);
		expect(r.pipe).toBe("2");
		await env.flush();
		expect(saveSetting).toHaveBeenCalledWith({ quickReply: { aiLimit: 2, globals: { g: "2" } } });
	});
	it("세트.버튼 이름으로 빠른 답장을 찾는다 (세트 이름에 점이 있어도)", () => {
		expect(quickReplySource("지.시.인사")).toBe("/echo 안녕");
		expect(quickReplySource("없는.것")).toBeUndefined();
	});
});
