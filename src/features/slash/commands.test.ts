import { beforeEach, describe, expect, it, vi } from "vitest";

// 앱 연결부(5-E-20-6-2) 시험: 가짜 IPC·저장소
const call = vi.fn(async (v: unknown) => v);
const patchMeta = vi.fn(async () => {});
const saveSetting = vi.fn(async () => {});
const setOptions = vi.fn(async () => {});
const conv = { id: "c", characterId: "ch", personaId: "p", meta: { variables: { hp: "10", list: '["a"]' } } };
const setting = { value: { quickReply: { aiLimit: 2, globals: { g: "1" } } } as Record<string, unknown>, save: saveSetting };

vi.mock("$shared/ipc/client", () => ({ call: (p: unknown) => call(p), commands: { slashGenerate: (_c: string, t: string) => ({ text: `AI:${t}`, truncated: t.includes("잘림") }), appendMessage: (m: unknown) => m } }));
const chooseFrom = vi.fn(async (_r: unknown) => "나");
const confirmAction = vi.fn(async (_r: unknown) => true);
vi.mock("$shared/stores/confirm.svelte", () => ({ promptText: vi.fn(), chooseFrom: (r: unknown) => chooseFrom(r), confirmAction: (r: unknown) => confirmAction(r) }));
const show = vi.fn();
vi.mock("$shared/stores/toast.svelte", () => ({ toasts: { info: vi.fn(), error: vi.fn(), success: vi.fn(), show: (...a: unknown[]) => show(...a) } }));
vi.mock("$shared/stores/aiTools.svelte", () => ({ pluginSetting: setting }));
vi.mock("$features/character/library.svelte", () => ({
	library: {
		conversation: () => conv,
		character: () => ({ name: "세라" }),
		persona: () => ({ name: "민" }),
		personas: [{ id: "p2", name: "여행자" }],
		presets: [{ id: "pr", name: "기본 배치" }],
		setOptions,
		patchMeta,
		refresh: vi.fn(),
	},
}));
const saveSet = vi.fn(async (_i: unknown) => {});
vi.mock("./quickReply.svelte", () => ({
	quickReplies: { loaded: true, load: vi.fn(), save: (i: unknown) => saveSet(i), sets: [{ id: "s1", name: "지.시", enabled: true, items: [{ label: "인사", message: "/echo 안녕" }] }] },
	toInput: (s: unknown) => s,
}));
vi.mock("$shared/stores/plugins.svelte", () => ({ plugins: { on: () => false } }));

const { quickReplySource, slashEnv } = await import("./commands");
const { runScript } = await import("./script");
const removeMany = vi.fn(async () => 0);
const msgs = [
	{ id: "m1", seq: 1, role: "user", name: null, content: "안녕" },
	{ id: "m2", seq: 2, role: "assistant", name: null, content: "반가워요" },
	{ id: "m3", seq: 3, role: "assistant", name: "카이", content: "나도 있어" },
];
const sendFn = vi.fn();
const continueLast = vi.fn();
const session = { convId: "c", busy: false, messages: msgs, send: sendFn, continueLast, regenerate: vi.fn(), removeMany } as never;

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
	it("/buttons는 고른 이름, /popup은 확인 1, /echo는 severity·title·timeout을 쓴다", async () => {
		const env = slashEnv(session);
		expect((await runScript('/buttons labels=["가","나"] 어느 쪽?', env)).pipe).toBe("나");
		expect(chooseFrom).toHaveBeenCalledWith({ title: "어느 쪽?", choices: ["가", "나"], multiple: false });
		expect((await runScript("/popup okButton=좋아 알려줄게", env)).pipe).toBe("1");
		expect(confirmAction).toHaveBeenCalledWith(expect.objectContaining({ message: "알려줄게", confirmLabel: "좋아", hideCancel: true }));
		await runScript("/echo severity=warning title=조심 timeout=1000 미끄러움", env);
		expect(show).toHaveBeenCalledWith({ tone: "warning", message: "조심 · 미끄러움", source: "슬래시 커맨드" }, 1000);
		await expect(runScript("/buttons 라벨 없음", env)).rejects.toThrow("labels");
	});
	it("/messages·/cut (0부터, 이름 붙이기), /comment는 AI에게 안 보내는 메시지", async () => {
		const env = slashEnv(session);
		expect((await runScript("/messages 1-2", env)).pipe).toBe("세라: 반가워요\n\n카이: 나도 있어");
		expect((await runScript("/messages names=off", env)).pipe).toBe("나도 있어");
		expect((await runScript("/cut 0", env)).pipe).toBe("안녕");
		expect(removeMany).toHaveBeenCalledWith(["m1"]);
		await runScript("/comment 메모", env);
		expect(call).toHaveBeenLastCalledWith(expect.objectContaining({ role: "system", content: "메모", meta: { hidden: true, comment: true } }));
	});
	it("/inject는 대화 meta.injects에, /flushinject는 비움, /genraw는 맥락 없이", async () => {
		const env = slashEnv(session);
		await runScript("/inject id=날씨 position=chat depth=2 비가 온다", env);
		expect(patchMeta).toHaveBeenLastCalledWith("c", { injects: { 날씨: { value: "비가 온다", position: "chat", depth: 2, role: "system" } } });
		await expect(runScript("/inject position=어딘가 x", env)).rejects.toThrow("position");
		await runScript("/flushinject", env);
		expect(patchMeta).toHaveBeenLastCalledWith("c", { injects: {} });
		expect((await runScript("/genraw system=요약가 한 줄로", env)).pipe).toBe("AI:한 줄로");
	});
	it("/persona·/preset은 이름으로 바꾸고, 없는 이름은 오류, /qr-set-off·/qr-chat-set-off", async () => {
		const env = slashEnv(session);
		expect((await runScript("/persona 여행자", env)).pipe).toBe("여행자");
		expect(setOptions).toHaveBeenLastCalledWith("c", expect.objectContaining({ personaId: "p2" }));
		expect((await runScript("/preset 기본 배치", env)).pipe).toBe("기본 배치");
		await expect(runScript("/persona 없는사람", env)).rejects.toThrow("찾지 못함");
		await runScript("/qr-set-off 지.시", env);
		expect(saveSet).toHaveBeenLastCalledWith(expect.objectContaining({ id: "s1", enabled: false }));
		await runScript("/qr-chat-set-off 지.시", env);
		expect(patchMeta).toHaveBeenLastCalledWith("c", { quickReplyOff: ["s1"] });
		await expect(runScript("/impersonate", env)).rejects.toThrow("대필 플러그인");
	});
});

describe("잘린 응답 (5-E-20-6-6)", () => {
	it("/gen 결과가 응답 길이에서 잘리면 경고 알림 (글은 그대로 돌려줌)", async () => {
		show.mockClear();
		const env = slashEnv(session);
		expect((await runScript("/gen 잘림 시험", env)).pipe).toBe("AI:잘림 시험");
		expect(show).toHaveBeenCalledWith(expect.objectContaining({ tone: "warning", message: expect.stringContaining("응답 길이 한도에서 잘림") }));
	});
});

describe("/trigger와 /continue (5-E-26)", () => {
	it("/trigger는 빈 보내기(새 AI 메시지), /continue는 이어 쓰기", async () => {
		const env = slashEnv(session);
		await runScript("/trigger", env);
		expect(sendFn).toHaveBeenCalledWith("");
		expect(continueLast).not.toHaveBeenCalled();
		await runScript("/continue", env);
		expect(continueLast).toHaveBeenCalledTimes(1);
	});
});
