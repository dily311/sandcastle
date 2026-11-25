/*! *****************************************************************************
언어 설정 유틸 (ko / en)
***************************************************************************** */

import { addMessages, getLocaleFromNavigator, init, locale } from "svelte-i18n";
import ko from "../../locales/ko.json";
import en from "../../locales/en.json";

export type AppLocale = "ko" | "en";
const STORAGE_KEY = "sc.locale";

addMessages("ko", ko);
addMessages("en", en);

function detectInitialLocale(): AppLocale {
	try {
		const saved = localStorage.getItem(STORAGE_KEY);
		if (saved === "ko" || saved === "en") return saved;
	} catch {
		/* localStorage unavailable */
	}
	const nav = (getLocaleFromNavigator() ?? "ko").toLowerCase();
	return nav.startsWith("en") ? "en" : "ko";
}

const initialLocale = detectInitialLocale();

init({
	fallbackLocale: "ko",
	initialLocale,
});

if (typeof document !== "undefined") {
	document.documentElement.lang = initialLocale;
}

export function setAppLocale(next: AppLocale) {
	locale.set(next);
	try {
		localStorage.setItem(STORAGE_KEY, next);
	} catch {
		/* ignore */
	}
	if (typeof document !== "undefined") {
		document.documentElement.lang = next;
	}
}

export function getAppLocale(): AppLocale {
	try {
		const saved = localStorage.getItem(STORAGE_KEY);
		if (saved === "ko" || saved === "en") return saved;
	} catch {
		/* ignore */
	}
	return initialLocale;
}
