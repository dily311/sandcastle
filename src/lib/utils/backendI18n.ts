// 백엔드(Rust)에서 한국어로 넘어오는 오류·플러그인·작업 라벨을 현재 앱 언어(ko / en)에 맞게 변환한다.
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import type { CorePlugin, ErrorCode } from "$ipc/bindings";
import { getAppLocale } from "./i18n";

const HAS_HANGUL = /[가-힣]/;

function t(key: string, values?: Record<string, string | number>): string {
	const fn = get(_);
	return fn(key, values ? { values } : undefined);
}

/**
 * 백엔드에서 내려온 오류 응답(code + message + details)을 현재 UI 언어로 변환한다.
 * - 한국어(ko): 백엔드의 상세 한국어 메시지를 그대로 사용 (비어 있으면 코드별 기본 문구)
 * - 그 외 언어(en 등):
 *   1) 코어 플러그인 비활성화 오류(`details.plugin`)면 번역된 플러그인 이름으로 안내
 *   2) 메시지에 한글이 포함되어 있으면 `errors.<code>` 번역 문구로 치환
 *   3) 영문 원문 오류(예: 공급자 HTTP/API 메시지)는 그대로 유지
 */
export function localizeBackendError(err: { code?: ErrorCode | "UNKNOWN" | string; message?: string | null; details?: unknown }): string {
	const raw = (err.message ?? "").trim();
	const code = err.code || "UNKNOWN";

	// 플러그인 비활성화 오류는 details.plugin을 이용해 현재 언어로 조합
	const pluginId = (err.details as { plugin?: unknown } | null | undefined)?.plugin;
	if (typeof pluginId === "string" && pluginId) {
		const pluginName = localizeCorePluginName(pluginId, pluginId);
		return t("errors.pluginDisabled", { name: pluginName });
	}

	if (getAppLocale() === "ko") {
		if (raw) return raw;
		const key = `errors.${code}`;
		const translated = t(key);
		return translated === key ? t("errors.UNKNOWN") : translated;
	}

	// 비한국어 로케일: 한글이 섞여 있으면 ErrorCode 기반 번역 문구 사용
	if (!raw || HAS_HANGUL.test(raw)) {
		const key = `errors.${code}`;
		const translated = t(key);
		if (translated !== key) return translated;
		return t("errors.UNKNOWN");
	}

	return raw;
}

const pluginKey = (id: string) => id.replace(/\./g, "_");

export function localizeCorePluginName(id: string, fallback: string): string {
	const key = `corePlugins.items.${pluginKey(id)}.name`;
	const translated = t(key);
	return translated === key ? fallback : translated;
}

export function localizeCorePluginDesc(id: string, fallback: string): string {
	const key = `corePlugins.items.${pluginKey(id)}.desc`;
	const translated = t(key);
	return translated === key ? fallback : translated;
}

export function localizeCorePlugin(p: CorePlugin): { name: string; description: string } {
	return {
		name: localizeCorePluginName(p.id, p.name),
		description: localizeCorePluginDesc(p.id, p.description),
	};
}

export function localizeTaskFeature(feature: string | null | undefined): string {
	if (!feature) return t("tasks.feature.default");
	if (feature.startsWith("plugin:")) return t("tasks.feature.plugin");
	const key = `tasks.feature.${feature}`;
	const translated = t(key);
	return translated === key ? feature : translated;
}

export function localizeTaskStatus(status: string): string {
	const key = `tasks.status.${status}`;
	const translated = t(key);
	return translated === key ? status : translated;
}
