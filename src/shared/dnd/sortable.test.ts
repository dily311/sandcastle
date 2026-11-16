import { describe, expect, it } from "vitest";
import { moveItem } from "./sortable.svelte";

describe("moveItem", () => {
	it("앞뒤로 옮긴다", () => {
		expect(moveItem(["a", "b", "c", "d"], 0, 2)).toEqual(["b", "c", "a", "d"]);
		expect(moveItem(["a", "b", "c", "d"], 3, 1)).toEqual(["a", "d", "b", "c"]);
	});
	it("범위 밖이나 같은 자리는 그대로 (새 배열)", () => {
		const list = ["a", "b"];
		expect(moveItem(list, 0, 0)).toEqual(list);
		expect(moveItem(list, 0, 5)).toEqual(list);
		expect(moveItem(list, 0, 0)).not.toBe(list);
	});
});

describe("로어북 끌어서 순서 = 우선순위", () => {
	const mk = (id: string, priority: number) => ({ id, bookId: "b", keys: ["k"], secondaryKeys: [], content: "", enabled: true, priority, constant: false, comment: id });
	it("이웃 사이에 자리가 있으면 옮긴 것만 바꾼다", async () => {
		const { reprioritize } = await import("$features/lorebook/lorebook.svelte");
		const list = [mk("a", 100), mk("c", 50), mk("b", 80)];
		// b를 1번 자리로: a(100) b c(50) → b = 75
		const moved = [list[0], list[2], list[1]];
		const changed = reprioritize(moved, 1);
		expect(changed.map((e) => e.id)).toEqual(["b"]);
		expect(moved.map((e) => e.priority)).toEqual([100, 75, 50]);
	});
	it("자리가 없으면 전부 다시 매긴다", async () => {
		const { reprioritize } = await import("$features/lorebook/lorebook.svelte");
		const moved = [mk("a", 100), mk("b", 10), mk("c", 99)];
		reprioritize(moved, 1);
		expect(moved.map((e) => e.priority)).toEqual([30, 20, 10]);
	});
	it("맨 위·맨 아래로", async () => {
		const { reprioritize } = await import("$features/lorebook/lorebook.svelte");
		const top = [mk("x", 5), mk("a", 100)];
		reprioritize(top, 0);
		expect(top[0].priority).toBe(110);
		const bottom = [mk("a", 100), mk("x", 500)];
		reprioritize(bottom, 1);
		expect(bottom[1].priority).toBe(90);
	});
});
