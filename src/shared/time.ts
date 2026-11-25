// 시각 표시: 목록·메시지에는 상대 시각, 툴팁에는 전체 날짜
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { getAppLocale } from "$lib/utils/i18n";

const t = (key: string, values?: Record<string, string | number>) => get(_)(key, values ? { values } : undefined);

/** ms 타임스탬프 → "방금", "3분 전", "2시간 전", "4일 전", "2달 전", "1년 전" */
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

/** 툴팁용 전체 날짜·시각 */
export function absolute(at: number): string {
	const loc = getAppLocale() === "en" ? "en-US" : "ko-KR";
	return new Date(at).toLocaleString(loc, { dateStyle: "full", timeStyle: "medium" });
}
