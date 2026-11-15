// 대화 북마크: 메시지 안의 고른 글을 대화 meta.bookmarks에 저장 (오른쪽 패널 북마크 탭)
import { toasts } from "$shared/stores/toast.svelte";
import { library, type Conversation } from "./library.svelte";

export interface ChatBookmark {
	id: string;
	messageId: string;
	text: string;
	createdAt: number;
}

export function bookmarksOf(conv: Conversation | undefined): ChatBookmark[] {
	const list = (conv?.meta as { bookmarks?: ChatBookmark[] } | null)?.bookmarks;
	return Array.isArray(list) ? list : [];
}

export const bookmarks = {
	async add(conv: Conversation, messageId: string, text: string) {
		const b: ChatBookmark = { id: `bm-${Date.now().toString(36)}`, messageId, text: text.slice(0, 600), createdAt: Date.now() };
		await library.patchMeta(conv.id, { bookmarks: [...bookmarksOf(conv), b] });
		toasts.success("북마크함");
	},
	async remove(conv: Conversation, id: string) {
		await library.patchMeta(conv.id, { bookmarks: bookmarksOf(conv).filter((b) => b.id !== id) });
	},
};
