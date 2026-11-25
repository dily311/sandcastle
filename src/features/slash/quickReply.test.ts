import { describe, expect, it, vi } from "vitest";

vi.mock("$shared/ipc/client", () => ({ call: vi.fn(), commands: {} }));
vi.mock("$shared/stores/appSetting.svelte", () => ({ appSetting: { value: { selectedPreset: "app-preset" } } }));
vi.mock("$features/character/library.svelte", () => ({ library: { character: (id: string) => (id === "ch" ? { defaultPromptPresetId: null } : undefined) } }));
vi.mock("./commands", () => ({ isSlash: (t: string) => t.startsWith("/"), runSlash: vi.fn() }));

const { buttonsOf, runQuickReply, setsFor, setsInScope } = await import("./quickReply.svelte");

const item = (id: string, label: string, message: string, hidden = false) => ({ id, label, message, hidden });
const set = (id: string, scope: string, scopeId: string | null, items = [item(`${id}1`, `${id} 버튼`, "안녕")], enabled = true) =>
	({ id, name: `${id} 세트`, scope, scopeId, enabled, autoSend: false, items, sortOrder: 0, createdAt: 0, updatedAt: 0 }) as never;
const SETS = [set("g", "global", null), set("c", "character", "ch"), set("other", "character", "other-ch"), set("p", "preset", "app-preset"), set("q", "preset", "conv-preset"), set("off", "global", null, undefined, false)];
const conv = (meta: unknown = null, promptPresetId: string | null = null) => ({ id: "c1", characterId: "ch", promptPresetId, meta }) as never;

describe("빠른 답장 범위", () => {
	it("전역 + 이 캐릭터 + 이 대화의 프리셋(대화 → 캐릭터 → 앱 기본) 세트만, 꺼 둔 세트는 빼고", () => {
		expect(setsFor(conv(), SETS).map((s: { id: string }) => s.id)).toEqual(["g", "c", "p"]);
		expect(setsFor(conv(null, "conv-preset"), SETS).map((s: { id: string }) => s.id)).toEqual(["g", "c", "q"]);
	});
	it("대화에서 끈 세트(meta.quickReplyOff)는 빼되, 범위 목록에는 남는다 (다시 켤 수 있게)", () => {
		expect(setsFor(conv({ quickReplyOff: ["c"] }), SETS).map((s: { id: string }) => s.id)).toEqual(["g", "p"]);
		expect(setsInScope(conv({ quickReplyOff: ["c"] }), SETS).map((s: { id: string }) => s.id)).toContain("c");
	});
	it("검색: 이름·내용·세트 이름에서 모든 낱말을 찾는다, 숨긴 버튼은 빠짐", () => {
		const s = [set("a", "global", null, [item("1", "빠른 지시", "/input 내용? | /gen {{pipe}}"), item("2", "인사", "안녕 반가워"), item("3", "숨김", "x", true)])];
		expect(buttonsOf(s).map((b) => b.q.label)).toEqual(["빠른 지시", "인사"]);
		expect(buttonsOf(s, "gen").map((b) => b.q.label)).toEqual(["빠른 지시"]);
		expect(buttonsOf(s, "인사 반가워").map((b) => b.q.label)).toEqual(["인사"]);
		expect(buttonsOf(s, "없는말")).toEqual([]);
	});
	it("바로 보내기가 꺼진 세트의 글은 입력칸에 덧붙이기만 (보내지 않음)", async () => {
		let draft = "처음";
		const send = vi.fn();
		const g = SETS[0] as { items: never[] };
		await runQuickReply(g.items[0], SETS[0], { busy: false, send } as never, { get: () => draft, set: (v) => (draft = v) });
		expect(draft).toBe("처음\n안녕");
		expect(send).not.toHaveBeenCalled();
	});
});
