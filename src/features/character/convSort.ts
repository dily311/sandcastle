import { lastAt, type Conversation } from "./library.svelte";
import { viewed } from "./viewed.svelte";

export type ConvSortBy = "viewed" | "recent" | "created" | "name" | "messages";

export const CONV_SORT_LABEL: Record<ConvSortBy, string> = {
	viewed: "최근 본 순",
	recent: "마지막 업데이트",
	created: "생성 날짜",
	name: "이름",
	messages: "메시지 순",
};

export const CONV_SORT_KEYS = Object.keys(CONV_SORT_LABEL) as ConvSortBy[];

export function isConvSortBy(v: unknown): v is ConvSortBy {
	return typeof v === "string" && v in CONV_SORT_LABEL;
}

/**
 * 대화 비교 함수 (공통: 사이드바·대화 목록).
 * - desc=true(내림차순): 최근 본 순 / 최신 업데이트 / 최근 생성 / 이름 역순 / 메시지 많은 순
 * - desc=false(오름차순): 오래 전에 본 순 / 오래된 업데이트 / 오래된 생성 / 이름순(가→하) / 메시지 적은 순
 */
export function compareConversations(a: Conversation, b: Conversation, sortBy: ConvSortBy, desc: boolean): number {
	let v = 0;
	switch (sortBy) {
		case "viewed":
			v = (viewed.at[a.id] ?? 0) - (viewed.at[b.id] ?? 0) || lastAt(a) - lastAt(b);
			break;
		case "recent":
			v = lastAt(a) - lastAt(b);
			break;
		case "created":
			v = a.createdAt - b.createdAt;
			break;
		case "name":
			v = a.title.localeCompare(b.title, "ko");
			break;
		case "messages":
			v = a.messageCount - b.messageCount || lastAt(a) - lastAt(b);
			break;
	}
	return desc ? -v : v;
}
