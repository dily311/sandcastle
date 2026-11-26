// 시각·수 표시 (5-C-2-1): Intl을 한 곳에서 써 현재 화면 언어에 맞춘다.
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { getAppLocale } from "$lib/utils/i18n";

const t = (key: string, values?: Record<string, string | number>) => get(_)(key, values ? { values } : undefined);

function intlTag(): string {
	return getAppLocale().startsWith("ko") ? "ko-KR" : "en-US";
}

/** ms 타임스탬프 → 상대 시각 */
export function relative(at: number, now = Date.now()): string {
	const min = Math.floor((now - at) / 60_000);
	if (min < 1) return t("settings.time.justNow");
	if (min < 60) return t("settings.time.minutesAgo", { n: min });
	const h = Math.floor(min / 60);
	if (h < 24) return t("settings.time.hoursAgo", { n: h });
	const d = Math.floor(h / 24);
	if (d < 30) return t("settings.time.daysAgo", { n: d });
	const mo = Math.floor(d / 30);
	if (mo < 12) return t("settings.time.monthsAgo", { n: mo });
	return t("settings.time.yearsAgo", { n: Math.floor(mo / 12) });
}

/** 툴팁용 전체 날짜·시각 (`Intl.DateTimeFormat`) */
export function absolute(at: number): string {
	return new Intl.DateTimeFormat(intlTag(), { dateStyle: "full", timeStyle: "medium" }).format(new Date(at));
}

/** 숫자 서식 (`Intl.NumberFormat`) */
export function formatNumber(n: number): string {
	return new Intl.NumberFormat(intlTag()).format(n);
}
