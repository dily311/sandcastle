import { describe, expect, it } from "vitest";
import { applyOrder, moveId, tabSetKey } from "./panel.svelte";

const tabs = ["settings", "actions", "map", "items"].map((id) => ({ id }));

describe("패널 탭 순서", () => {
	it("탭 묶음 키는 순서와 상관없다", () => {
		expect(tabSetKey(tabs)).toBe(tabSetKey([...tabs].reverse()));
	});

	it("옮긴 순서를 적용하고, 저장에 없는 새 탭은 뒤에 붙인다", () => {
		const order = moveId(["settings", "actions", "map"], "map", "settings");
		expect(order).toEqual(["map", "settings", "actions"]);
		expect(applyOrder(tabs, order).map((t) => t.id)).toEqual(["map", "settings", "actions", "items"]);
	});

	it("저장된 순서가 없으면 그대로", () => {
		expect(applyOrder(tabs, null)).toBe(tabs);
	});
});
