import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { getAppLocale, setAppLocale } from "./i18n";
import ko from "../../locales/ko.json";
import en from "../../locales/en.json";

function flatten(obj: Record<string, unknown>, prefix = ""): string[] {
	return Object.entries(obj).flatMap(([k, v]) => {
		const key = prefix ? `${prefix}.${k}` : k;
		return v && typeof v === "object" && !Array.isArray(v) ? flatten(v as Record<string, unknown>, key) : [key];
	});
}

function walkFiles(dir: string): string[] {
	const out: string[] = [];
	for (const entry of readdirSync(dir)) {
		const full = path.join(dir, entry);
		const st = statSync(full);
		if (st.isDirectory()) out.push(...walkFiles(full));
		else if (/\.(svelte|ts)$/.test(entry) && !entry.endsWith(".test.ts")) out.push(full);
	}
	return out;
}

describe("i18n", () => {
	const koKeys = flatten(ko as Record<string, unknown>).sort();
	const enKeys = flatten(en as Record<string, unknown>).sort();

	it("ko.json과 en.json의 키가 모두 일치한다", () => {
		expect(enKeys).toEqual(koKeys);
	});

	it("소스 코드에서 $_()로 참조하는 모든 정적 키가 ko/en 사전에 존재한다", () => {
		const srcDir = path.resolve(import.meta.dirname, "../..");
		const koSet = new Set(koKeys);
		const enSet = new Set(enKeys);
		const missingKo: string[] = [];
		const missingEn: string[] = [];
		const re = /\$_\(\s*["']([^"'${}]+)["']/g;

		for (const file of walkFiles(srcDir)) {
			const content = readFileSync(file, "utf8");
			for (const m of content.matchAll(re)) {
				const key = m[1];
				if (!koSet.has(key)) missingKo.push(`${path.relative(srcDir, file)}: ${key}`);
				if (!enSet.has(key)) missingEn.push(`${path.relative(srcDir, file)}: ${key}`);
			}
		}

		expect(missingKo, "ko.json에 누락된 키").toEqual([]);
		expect(missingEn, "en.json에 누락된 키").toEqual([]);
	});

	it("언어 전환 시 번역 텍스트와 html lang이 바뀐다", () => {
		setAppLocale("ko");
		expect(getAppLocale()).toBe("ko");
		expect(document.documentElement.lang).toBe("ko");
		expect(get(_)("nav.settings")).toBe("설정");

		setAppLocale("en");
		expect(getAppLocale()).toBe("en");
		expect(document.documentElement.lang).toBe("en");
		expect(get(_)("nav.settings")).toBe("Settings");

		setAppLocale("ko");
	});

	it("변수 치환(interpolation)이 ko/en 모두 정상 동작한다", () => {
		setAppLocale("ko");
		expect(get(_)("common.selectedCount", { values: { count: 3, unit: "개" } })).toBe("3개 선택됨");
		expect(get(_)("common.deleteCount", { values: { count: 2, unit: "장" } })).toBe("2장 삭제");

		setAppLocale("en");
		expect(get(_)("common.selectedCount", { values: { count: 3, unit: "" } })).toBe("3 selected");
		expect(get(_)("common.deleteCount", { values: { count: 2, unit: "" } })).toBe("Delete 2");

		setAppLocale("ko");
	});
});
