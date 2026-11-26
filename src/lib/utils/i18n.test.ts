import { beforeAll, describe, expect, it } from "vitest";
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { AVAILABLE_LOCALES, defaultTargetLang, getAppLocale, initI18n, setAppLocale } from "./i18n";
import {
	localizeApiTestReply,
	localizeBackendError,
	localizeCorePlugin,
	localizeImportWarning,
	localizeQuotaLine,
	localizeTaskFeature,
	localizeTaskLabel,
	localizeTaskStatus,
} from "./backendI18n";

function flatten(obj: Record<string, unknown>, prefix = ""): string[] {
	return Object.entries(obj).flatMap(([k, v]) => {
		const key = prefix ? `${prefix}.${k}` : k;
		return v && typeof v === "object" && !Array.isArray(v) ? flatten(v as Record<string, unknown>, key) : [key];
	});
}

// 영역별 사전 파일 (모든 언어)
const LOCALE_MODULES = import.meta.glob("/src/locales/*/*.json", { eager: true, import: "default" }) as Record<
	string,
	Record<string, unknown>
>;

// 소스 파일 원문 (테스트·시안·가짜 백엔드 제외)
const SOURCES = import.meta.glob(
	["/src/**/*.svelte", "/src/**/*.ts", "!/src/**/*.test.ts", "!/src/ipc/dev-mock/**", "!/src/design-preview/**"],
	{ query: "?raw", import: "default", eager: true },
) as Record<string, string>;

describe("i18n (ADR-032, 5-C-2-1)", () => {
	const keysByLang = new Map<string, string[]>();
	const duplicatesByLang = new Map<string, string[]>();

	for (const lang of AVAILABLE_LOCALES) {
		const seen = new Set<string>();
		const dups: string[] = [];
		for (const [path, mod] of Object.entries(LOCALE_MODULES)) {
			if (!path.includes(`/locales/${lang}/`)) continue;
			for (const k of flatten(mod)) {
				if (seen.has(k)) dups.push(`${path}: ${k}`);
				seen.add(k);
			}
		}
		keysByLang.set(lang, Array.from(seen).sort());
		duplicatesByLang.set(lang, dups);
	}

	beforeAll(async () => {
		await initI18n();
	});

	it("지원 언어 폴더가 자동 인식되고 영역 파일 사이에 중복 키가 없다", () => {
		expect(AVAILABLE_LOCALES).toContain("ko");
		expect(AVAILABLE_LOCALES).toContain("en");
		for (const [lang, dups] of duplicatesByLang.entries()) {
			expect(dups, `${lang} 영역 파일 간 중복 키`).toEqual([]);
		}
	});

	it("모든 지원 언어의 전체 키 집합이 일치한다", () => {
		const baseKeys = keysByLang.get("ko") ?? [];
		expect(baseKeys.length).toBeGreaterThan(3000);
		for (const lang of AVAILABLE_LOCALES) {
			if (lang === "ko") continue;
			expect(keysByLang.get(lang), `${lang}과 ko의 키 집합 불일치`).toEqual(baseKeys);
		}
	});

	it("소스 코드에서 $_()로 참조하는 모든 정적 키가 모든 언어 사전에 존재한다", () => {
		const re = /\$_\(\s*["']([^"'${}]+)["']/g;
		for (const lang of AVAILABLE_LOCALES) {
			const set = new Set(keysByLang.get(lang) ?? []);
			const missing: string[] = [];
			for (const [file, content] of Object.entries(SOURCES)) {
				for (const m of content.matchAll(re)) {
					const key = m[1];
					if (!set.has(key)) missing.push(`${file}: ${key}`);
				}
			}
			expect(missing, `${lang} 사전에 누락된 키`).toEqual([]);
		}
	});

	it("언어 지연 전환 시 번역 텍스트, html lang, 기본 번역 대상 언어가 바뀐다", async () => {
		await setAppLocale("ko", { persistSetting: false });
		expect(getAppLocale()).toBe("ko");
		expect(document.documentElement.lang).toBe("ko");
		expect(get(_)("nav.settings")).toBe("설정");
		expect(defaultTargetLang()).toBe("한국어");

		await setAppLocale("en", { persistSetting: false });
		expect(getAppLocale()).toBe("en");
		expect(document.documentElement.lang).toBe("en");
		expect(get(_)("nav.settings")).toBe("Settings");
		expect(defaultTargetLang()).toBe("English");

		await setAppLocale("ko", { persistSetting: false });
	});

	it("변수 치환(interpolation)이 ko/en 모두 정상 동작하고 모든 키가 ICU 문법 오류·한글 누출 없이 렌더링된다", async () => {
		for (const lang of ["ko", "en"] as const) {
			await setAppLocale(lang, { persistSetting: false });
			const translate = get(_);
			const badRender: string[] = [];
			const leakedHangul: string[] = [];

			for (const [path, mod] of Object.entries(LOCALE_MODULES)) {
				if (!path.includes(`/locales/${lang}/`)) continue;
				const checkEntries = (obj: Record<string, unknown>, prefix = "") => {
					for (const [k, raw] of Object.entries(obj)) {
						const key = prefix ? `${prefix}.${k}` : k;
						if (typeof raw === "string") {
							const vars = [...raw.matchAll(/(?<!\{)\{([A-Za-z0-9_]+)\}(?!\})/g)].map((m) => m[1]);
							const values = Object.fromEntries(vars.map((v) => [v, `__VAL_${v}__`]));
							const rendered = translate(key, vars.length ? { values } : undefined);
							for (const varName of vars) {
								if (!rendered.includes(`__VAL_${varName}__`)) {
									badRender.push(`${lang}:${key} -> placeholder {${varName}} not interpolated (got: ${rendered})`);
								}
							}
							// settings.tabKeywords.*는 화면에 보이지 않는 한/영 병기 검색 색인(영문 화면에서 '언어' 검색 복귀용)
							if (lang === "en" && !key.startsWith("settings.tabKeywords.") && /[가-힣]/.test(rendered)) {
								leakedHangul.push(`${key}: ${rendered}`);
							}
						} else if (raw && typeof raw === "object" && !Array.isArray(raw)) {
							checkEntries(raw as Record<string, unknown>, key);
						}
					}
				};
				checkEntries(mod);
			}

			expect(badRender, `${lang} ICU 변수 치환 실패`).toEqual([]);
			if (lang === "en") {
				expect(leakedHangul, "en 사전에 한글 잔존").toEqual([]);
			}
		}

		await setAppLocale("ko", { persistSetting: false });
		expect(get(_)("common.selectedCount", { values: { count: 3, unit: "개" } })).toBe("3개 선택됨");
		expect(get(_)("common.deleteCount", { values: { count: 2, unit: "장" } })).toBe("2장 삭제");

		await setAppLocale("en", { persistSetting: false });
		expect(get(_)("common.selectedCount", { values: { count: 3, unit: "" } })).toBe("3 selected");
		expect(get(_)("common.deleteCount", { values: { count: 2, unit: "" } })).toBe("Delete 2");

		await setAppLocale("ko", { persistSetting: false });
	});

	it("백엔드에서 넘어온 오류·플러그인·작업 라벨을 현재 언어에 맞게 변환한다", async () => {
		await setAppLocale("en", { persistSetting: false });
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
		expect(
			localizeBackendError({
				code: "INVALID_INPUT",
				message: "캐릭터를 찾을 수 없습니다: char-42",
				key: "errors.detail.charNotFoundId",
				args: { id: "char-42" },
			}),
		).toBe("Character not found: char-42");
		expect(
			localizeBackendError({
				code: "INVALID_INPUT",
				message: "플러그인 manifest.json: apiVersion이 없음",
				key: "errors.detail.pluginManifestPrefix",
				args: { msg: "apiVersion이 없음", subKey: "errors.detail.manifestMissingApiVersion" },
			}),
		).toBe("Plugin manifest.json: Missing apiVersion");
		expect(
			localizeBackendError({
				code: "AI_FEATURE_UNSUPPORTED",
				message: "'내 Claude' 연결은 임베딩을(를) 지원하지 않음: 이 공급자에는 임베딩 API가 없음",
				key: "errors.detail.capUnsupportedConn",
				args: { profile: "내 Claude", feature: "임베딩", reason: "이 공급자에는 임베딩 API가 없음" },
			}),
		).toBe("Connection '내 Claude' does not support embeddings: this provider has no embeddings API");
		expect(localizeTaskLabel("응답 생성")).toBe("Generate reply");
		expect(localizeTaskLabel("번역 → English")).toBe("Translate → English");
		expect(localizeTaskLabel("자동 재생성 (1/2)")).toBe("Auto rewrite (1/2)");
		expect(localizeQuotaLine({ label: "구독", value: "무료" })).toEqual({ label: "Subscription", value: "Free" });
		expect(localizeApiTestReply("모델 12개 확인됨")).toBe("12 models verified");
		expect(localizeImportWarning("정규식 '테스트': 찾을 패턴이 비어 있어 건너뜀")).toBe("Regex '테스트': skipped because find pattern is empty");

		await setAppLocale("ko", { persistSetting: false });
		expect(
			localizeBackendError({
				code: "AI_PROVIDER_RATE_LIMIT",
				message: "요청 한도를 초과했습니다",
			}),
		).toBe("요청 한도를 초과했습니다");
		expect(localizeTaskFeature("chat")).toBe("대화");
	});
});
