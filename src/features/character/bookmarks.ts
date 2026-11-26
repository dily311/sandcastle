// 대화 북마크: 메시지 안의 고른 글을 대화 meta.bookmarks에 저장 (오른쪽 패널 북마크 탭)
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { toasts } from "$shared/stores/toast.svelte";
import { library, type Conversation } from "./library.svelte";

export interface ChatBookmark {
	id: string;
	messageId: string;
	text: string;
	createdAt: number;
	/** 북마크한 때 보고 있던 스와이프 (없으면 예전 북마크: 어느 스와이프든 보임) */
	swipeId?: number;
}

export function bookmarksOf(conv: Conversation | undefined): ChatBookmark[] {
	const list = (conv?.meta as { bookmarks?: ChatBookmark[] } | null)?.bookmarks;
	return Array.isArray(list) ? list : [];
}

/** 지금 보이는 스와이프의 북마크인가 (스와이프 번호가 없는 예전 북마크는 늘 해당) */
export const onCurrentSwipe = (b: ChatBookmark, currentSwipe: number) => b.swipeId === undefined || b.swipeId === currentSwipe;

export const bookmarks = {
	async add(conv: Conversation, messageId: string, text: string, swipeId = 0) {
		const b: ChatBookmark = { id: `bm-${Date.now().toString(36)}`, messageId, text: text.slice(0, 600), createdAt: Date.now(), swipeId };
		await library.patchMeta(conv.id, { bookmarks: [...bookmarksOf(conv), b] });
		toasts.success(get(_)("chatExtra.bookmarkedToast"));
	},
	async remove(conv: Conversation, id: string) {
		await library.patchMeta(conv.id, { bookmarks: bookmarksOf(conv).filter((b) => b.id !== id) });
	},
};
