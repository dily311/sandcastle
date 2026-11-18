import { describe, expect, it } from "vitest";
import { drops } from "./drop.svelte";

const t = (label: string, extra: object = {}) => ({ label, accept: () => true, ondrop: () => {}, ...extra });

describe("drops.targetAt: 영역별 받기", () => {
	it("영역 안에서는 그 영역의 대상이, 밖에서는 영역 없는 대상이 받는다", () => {
		const a = document.createElement("div");
		const inA = document.createElement("span");
		a.appendChild(inA);
		const outside = document.createElement("div");
		document.body.append(a, outside);
		const offAll = drops.register(t("전체"));
		const offA = drops.register(t("A 영역", { zone: () => a }));
		expect(drops.targetAt(inA)?.label).toBe("A 영역");
		expect(drops.targetAt(outside)?.label).toBe("전체");
		offA();
		expect(drops.targetAt(inA)?.label).toBe("전체");
		offAll();
	});
	it("영역만 있는 대상은 영역 밖에서 받지 않는다", () => {
		const a = document.createElement("div");
		const outside = document.createElement("div");
		document.body.append(a, outside);
		const off = drops.register(t("읽기", { zone: () => a }));
		expect(drops.targetAt(a)?.label).toBe("읽기");
		expect(drops.targetAt(outside)).toBeNull();
		expect(drops.targetAt(null)).toBeNull();
		off();
	});
	it("겹치는 영역은 priority가 높은 것", () => {
		const a = document.createElement("div");
		document.body.append(a);
		const o1 = drops.register(t("낮음", { zone: () => a, priority: 1 }));
		const o2 = drops.register(t("높음", { zone: () => a, priority: 2 }));
		expect(drops.targetAt(a)?.label).toBe("높음");
		o1();
		o2();
	});
});
