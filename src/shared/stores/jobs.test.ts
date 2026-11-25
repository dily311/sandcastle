import { beforeEach, describe, expect, it, vi } from "vitest";

const show = vi.fn();
vi.mock("./toast.svelte", () => ({ toasts: { show: (...a: unknown[]) => show(...a) } }));

const { jobs, SKIPPED } = await import("./jobs.svelte");

/** 밖에서 끝낼 수 있는 작업 */
function gate<T = void>() {
	let open!: (v: T) => void;
	let fail!: (e: unknown) => void;
	const p = new Promise<T>((res, rej) => ((open = res), (fail = rej)));
	return { p, open, fail };
}
const tick = () => new Promise((r) => setTimeout(r, 0));

describe("파일 작업 대기열", () => {
	beforeEach(() => show.mockClear());

	it("하나씩 차례로 돌고, 도는 동안 누른 것은 뒤에 붙는다", async () => {
		const order: string[] = [];
		const a = gate();
		const first = jobs.add("A", async () => (order.push("A 시작"), await a.p, order.push("A 끝")));
		const second = jobs.add("B", async () => void order.push("B"));
		await tick();
		expect(order).toEqual(["A 시작"]);
		expect(jobs.current?.label).toBe("A");
		expect(jobs.waiting.map((w) => w.label)).toEqual(["B"]);
		expect(jobs.pending).toBe(2);
		a.open();
		await Promise.all([first, second]);
		expect(order).toEqual(["A 시작", "A 끝", "B"]);
		expect(jobs.pending).toBe(0);
		await tick();
		// 두 개를 했으니 요약 한 번
		expect(show).toHaveBeenCalledOnce();
		expect(show.mock.calls[0][0].message).toContain("A 1개");
	});

	it("실패해도 다음 것을 하고, 실패는 요약에 남는다", async () => {
		const failing = jobs.add("지우기", async () => Promise.reject(new Error("사용 중")), { group: "캐릭터 지우기" });
		const next = jobs.add("지우기", async () => 7, { group: "캐릭터 지우기" });
		await expect(failing).rejects.toThrow("사용 중");
		expect(await next).toBe(7);
		await tick();
		expect(show.mock.calls[0][0]).toMatchObject({ tone: "warning", message: "캐릭터 지우기 1개 · 실패 1개" });
	});

	it("기다리는 것은 뺄 수 있고, 뺀 것은 SKIPPED로 끝난다", async () => {
		const a = gate();
		const first = jobs.add("A", () => a.p);
		const ran = vi.fn();
		const second = jobs.add("B", async () => ran());
		await tick();
		jobs.remove(jobs.waiting[0].id);
		expect(jobs.waiting).toEqual([]);
		expect(await second).toBe(SKIPPED);
		a.open();
		await first;
		expect(ran).not.toHaveBeenCalled();
	});

	it("멈출 수 있는 작업만 멈춤 신호를 받는다", async () => {
		const a = gate();
		let saw = false;
		const job = jobs.add("가져오기", async (ctx) => (await a.p, (saw = ctx.stopped())), { stoppable: true });
		await tick();
		jobs.stop();
		a.open();
		await job;
		expect(saw).toBe(true);

		const b = gate();
		let saw2 = true;
		const plain = jobs.add("지우기", async (ctx) => (await b.p, (saw2 = ctx.stopped())));
		await tick();
		jobs.stop(); // 멈출 수 없는 것은 무시
		b.open();
		await plain;
		expect(saw2).toBe(false);
	});

	it("하나뿐이고 금방 끝나면 막대도 요약도 없다", async () => {
		const job = jobs.add("A", async () => 1);
		expect(jobs.visible).toBe(false);
		await job;
		await tick();
		expect(show).not.toHaveBeenCalled();
	});
});
