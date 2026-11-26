/*! *****************************************************************************
언어 설정 유틸 (ADR-032, 5-C-2-1)
- 영역별 사전(`src/locales/<언어>/<영역>.json`)을 현재 언어만 지연 불러온다 (Lazy Initialization).
- 언어 폴더를 추가하면 자동으로 지원 언어 목록에 잡힌다.
- 기기 전용 `AppSetting.language`와 초기 부팅용 `localStorage("sc.locale")`·`<html lang>`을 동기화한다.
- 저장된 언어가 없을 때 초기 기본값은 OS/브라우저 언어가 `ko*`면 `ko`, 그 밖은 `en`.
***************************************************************************** */

import { get } from "svelte/store";
import { _, getLocaleFromNavigator, init, locale, register, waitLocale } from "svelte-i18n";

export { _ };

/** 컴포넌트 밖(TS 모듈·스토어·유틸)에서 현재 로케일 문구를 즉시 꺼낸다 */
export function tr(
	key: string,
	values?: Record<string, string | number | boolean | Date | null | undefined>,
): string {
	return get(_)(key, values ? { values } : undefined);
}

const LOCALE_LOADERS = import.meta.glob("../../locales/*/*.json") as Record<
	string,
	() => Promise<{ default: Record<string, unknown> }>
>;

/** `src/locales/<lang>/*.json` 폴더 이름에서 자동 수집된 지원 언어 목록 */
export const AVAILABLE_LOCALES: string[] = Array.from(
	new Set(
		Object.keys(LOCALE_LOADERS)
			.map((p) => p.match(/\/locales\/([^/]+)\/[^/]+\.json$/)?.[1])
			.filter((v): v is string => Boolean(v)),
	),
).sort();

export type AppLocale = "ko" | "en" | (string & {});

const STORAGE_KEY = "sc.locale";

function isSupportedLocale(v: string | null | undefined): v is AppLocale {
	return Boolean(v && AVAILABLE_LOCALES.includes(v));
}

function deepMerge(target: Record<string, unknown>, source: Record<string, unknown>): Record<string, unknown> {
	for (const [k, v] of Object.entries(source)) {
		if (v && typeof v === "object" && !Array.isArray(v) && target[k] && typeof target[k] === "object" && !Array.isArray(target[k])) {
			deepMerge(target[k] as Record<string, unknown>, v as Record<string, unknown>);
		} else {
			target[k] = v;
		}
	}
	return target;
}

// 각 언어별로 자기 폴더의 영역 JSON들만 필요할 때 불러와 합친다
for (const lang of AVAILABLE_LOCALES) {
	const prefix = `/locales/${lang}/`;
	const loaders = Object.entries(LOCALE_LOADERS)
		.filter(([path]) => path.includes(prefix))
		.map(([, load]) => load);
	register(lang, async () => {
		const modules = await Promise.all(loaders.map((fn) => fn()));
		const merged: Record<string, unknown> = {};
		for (const mod of modules) {
			deepMerge(merged, mod.default ?? (mod as unknown as Record<string, unknown>));
		}
		return merged;
	});
}

export function hasSavedLocaleInStorage(): boolean {
	try {
		return isSupportedLocale(localStorage.getItem(STORAGE_KEY));
	} catch {
		return false;
	}
}

export function detectOsLocale(): AppLocale {
	const raw = (getLocaleFromNavigator() ?? "en").toLowerCase();
	if (raw.startsWith("ko")) return "ko";
	const base = raw.split("-")[0];
	if (isSupportedLocale(base)) return base;
	return "en";
}

function detectInitialLocale(): AppLocale {
	try {
		const saved = localStorage.getItem(STORAGE_KEY);
		if (isSupportedLocale(saved)) return saved;
	} catch {
		/* localStorage unavailable */
	}
	return detectOsLocale();
}

let currentLocaleCache: AppLocale = detectInitialLocale();
let persistCallback: ((next: AppLocale) => void) | null = null;

/** `appSetting` 저장소가 준비되면 등록해 언어 변경을 백엔드 설정(`AppSetting.language`)에도 남긴다 */
export function onLocalePersist(cb: (next: AppLocale) => void) {
	persistCallback = cb;
}

init({
	fallbackLocale: "en",
	initialLocale: currentLocaleCache,
});

if (typeof document !== "undefined") {
	document.documentElement.lang = currentLocaleCache;
}

let bootPromise: Promise<AppLocale> | null = null;

/** 앱/위젯 마운트 전에 현재 언어 사전이 준비될 때까지 기다린다 */
export function initI18n(): Promise<AppLocale> {
	bootPromise ??= waitLocale(currentLocaleCache).then(() => currentLocaleCache);
	return bootPromise;
}

export async function setAppLocale(next: AppLocale, opts?: { persistSetting?: boolean }): Promise<void> {
	await initI18n();
	const resolved: AppLocale = isSupportedLocale(next) ? next : "en";
	currentLocaleCache = resolved;
	try {
		localStorage.setItem(STORAGE_KEY, resolved);
	} catch {
		/* ignore */
	}
	if (typeof document !== "undefined") {
		document.documentElement.lang = resolved;
	}
	locale.set(resolved);
	await waitLocale(resolved);
	if (opts?.persistSetting !== false) {
		persistCallback?.(resolved);
	}
}

export function getAppLocale(): AppLocale {
	try {
		const saved = localStorage.getItem(STORAGE_KEY);
		if (isSupportedLocale(saved)) {
			currentLocaleCache = saved;
			return saved;
		}
	} catch {
		/* ignore */
	}
	return currentLocaleCache;
}

/** 번역 플러그인의 `targetLang`이 비어 있을 때 화면 언어를 따라가는 기본 언어 이름 (ADR-032 ⑥) */
export function defaultTargetLang(loc: AppLocale = getAppLocale()): string {
	if (loc.startsWith("ko")) return "한국어"; // i18n-ignore: AI 번역 지시용 대상 언어명
	if (loc.startsWith("en")) return "English";
	try {
		return new Intl.DisplayNames(["en"], { type: "language" }).of(loc) ?? "English";
	} catch {
		return "English";
	}
}
