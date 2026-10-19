/*! *****************************************************************************
luxon 라이브러리 wrapper
***************************************************************************** */

import { DateTime } from "luxon"

// 포맷에 따라 호출하는 함수가 다르게 할까?
// 아니면 매개변수를 하나 더 받아서 원하는 형식을 작성하는 식으로 하는 게 좋을까?
export function currentLocale(): string {
	if (typeof window === "undefined") return "en-US";
	return window.navigator.language;
}

// Token 확인: https://moment.github.io/luxon/#/parsing?id=table-of-tokens
export function formatDateTime(timestep: number, tokens: string = "ff"): string {
    const datetime = DateTime.fromMillis(timestep);
    return datetime.toFormat(tokens) ?? "";
}

export function formatRelativeTime(timestamp: number): string {
    const now = DateTime.now();
    const datetime = DateTime.fromMillis(timestamp);
    // 표기된 날짜가 미래인 경우 
    if (datetime > now) {
        return datetime.toFormat("D") ?? "";
    }
	const relative = datetime.toLocal().toRelativeCalendar({ base: now, locale: "ko"});
    return relative ?? "";
}