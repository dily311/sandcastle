import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { getAppLocale, setAppLocale } from "./i18n";
import { localizeBackendError, localizeCorePlugin, localizeTaskFeature, localizeTaskStatus } from "./backendI18n";
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

	it("백엔드에서 넘어온 한국어 오류·플러그인·작업 라벨을 현재 언어에 맞게 변환한다", () => {
		setAppLocale("en");
		expect(
			localizeBackendError({
				code: "AI_PROVIDER_RATE_LIMIT",
				message: "요청 한도를 초과했습니다",
			}),
		).toBe("AI rate limit exceeded. Please try again later.");
		expect(
			localizeBackendError({
				code: "INVALID_INPUT",
				message: "'번역' 기능이 꺼져 있습니다 (설정 > 코어 플러그인)",
				details: { plugin: "translate" },
			}),
		).toBe("'Translate' is disabled (Settings > Core Plugins)");
		expect(
			localizeBackendError({
				code: "AI_PROVIDER_ERROR",
				message: "Upstream HTTP 502 Bad Gateway",
			}),
		).toBe("Upstream HTTP 502 Bad Gateway");

		const pluginLoc = localizeCorePlugin({
			id: "phone",
			name: "폰",
			description: "캐릭터와 메신저처럼 대화하는 폰 화면",
			defaultEnabled: true,
			desktopOnly: false,
			enabled: true,
		});
		expect(pluginLoc.name).toBe("Phone");
		expect(pluginLoc.description).toContain("Messenger-style phone screen");
		expect(localizeTaskFeature("chat")).toBe("Chat");
		expect(localizeTaskStatus("running")).toBe("Running");

		setAppLocale("ko");
		expect(
			localizeBackendError({
				code: "AI_PROVIDER_RATE_LIMIT",
				message: "요청 한도를 초과했습니다",
			}),
		).toBe("요청 한도를 초과했습니다");
		expect(localizeTaskFeature("chat")).toBe("대화");
	});
});
