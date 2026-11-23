<script lang="ts" module>
	export interface CardInput {
		text: string;
		speaker?: string;
		time?: number;
	}
</script>

<script lang="ts">
	// 글을 끌어서 고르면: 북마크 / 복사 / 이미지 카드 (대화 화면·읽기 화면이 같이 씀, 5-E-6-2)
	// 마우스는 고른 뒤 오른쪽 클릭, 손가락은 손을 떼고 고른 범위가 멈춘 뒤 저절로
	import { Bookmark, Copy, IdCard } from "@lucide/svelte";
	import { ContextMenu } from "$shared/ui";
	import { toasts } from "$shared/stores/toast.svelte";
	import { bookmarks } from "./bookmarks";
	import type { Conversation } from "./library.svelte";
	import { metaOf, type ChatSession } from "./session.svelte";

	interface Props {
		/** 메시지들이 든 스크롤 칸 (이 안의 메시지 본문만) */
		scroller: HTMLElement | undefined;
		conv: Conversation | undefined;
		session: ChatSession | null;
		/** AI 메시지에 이름이 없을 때 말한 사람 */
		characterName?: string;
		/** 여러 메시지 고르기 중에는 띄우지 않음 */
		disabled?: boolean;
		oncard: (c: CardInput) => void;
	}
	let { scroller, conv, session, characterName, disabled = false, oncard }: Props = $props();

	let selMenu = $state<{ x: number; y: number; text: string; messageId: string } | null>(null);
	/** 본문이 아닌 곳: 머리줄(번호·이름·시각)·도구 줄·번역 안내·접기·CG 그림 이름 */
	const inMeta = (n: Element | null) => !!n?.closest(".who, .actions, .swipe, .tr-note, .fold, figcaption");
	/** 고른 글 중 한 메시지 본문에 든 부분. 끝까지 끌어 도구 줄·다음 칸까지 넘어가도 본문만 잘라 쓴다 */
	function pickedText(): { text: string; messageId: string; rect: DOMRect } | null {
		const sel = window.getSelection();
		if (!sel || sel.isCollapsed || !sel.rangeCount) return null;
		const el = (n: Node | null) => (n instanceof Element ? n : (n?.parentElement ?? null));
		if (el(sel.anchorNode)?.closest("textarea, input, [contenteditable='true']")) return null;
		const block = el(sel.anchorNode)?.closest<HTMLElement>("[id^=msg-]") ?? el(sel.focusNode)?.closest<HTMLElement>("[id^=msg-]");
		if (!block || !scroller?.contains(block)) return null;
		const range = sel.getRangeAt(0);
		let text = "";
		let lastLine: Element | null = null;
		const walker = document.createTreeWalker(block, NodeFilter.SHOW_TEXT);
		for (let n = walker.nextNode(); n; n = walker.nextNode()) {
			if (!range.intersectsNode(n) || inMeta(n.parentElement)) continue;
			const raw = n.textContent ?? "";
			const s = n === range.startContainer ? range.startOffset : 0;
			const e = n === range.endContainer ? range.endOffset : raw.length;
			const line = n.parentElement?.closest("p, h2, h3, h4, li, blockquote, pre") ?? null;
			if (lastLine && line !== lastLine) text += "\n";
			lastLine = line;
			text += raw.slice(s, e);
		}
		text = text.trim();
		return text ? { text, messageId: block.id.slice(4), rect: range.getBoundingClientRect() } : null;
	}
	/** 손가락: 꾹 눌러 고르는 동안 메뉴를 띄우면 포커스가 옮겨 가 고르기가 끊긴다 → 손을 떼고 고른 범위가 멈춘 뒤에 띄운다 */
	const coarse = typeof matchMedia !== "undefined" && matchMedia("(pointer: coarse)").matches;
	let touching = false;
	let selTimer: ReturnType<typeof setTimeout> | undefined;
	function menuAfterSettle() {
		clearTimeout(selTimer);
		selTimer = setTimeout(() => {
			if (touching || disabled || selMenu) return;
			const p = pickedText();
			if (p) selMenu = { x: p.rect.left + p.rect.width / 2, y: p.rect.bottom + 8, text: p.text, messageId: p.messageId };
		}, 450);
	}
	$effect(() => {
		if (!coarse) return;
		const down = () => {
			touching = true;
			clearTimeout(selTimer);
		};
		const up = () => {
			touching = false;
			menuAfterSettle();
		};
		const change = () => {
			if (!touching) menuAfterSettle();
		};
		document.addEventListener("touchstart", down, { passive: true });
		document.addEventListener("touchend", up, { passive: true });
		document.addEventListener("touchcancel", up, { passive: true });
		document.addEventListener("selectionchange", change);
		return () => {
			clearTimeout(selTimer);
			document.removeEventListener("touchstart", down);
			document.removeEventListener("touchend", up);
			document.removeEventListener("touchcancel", up);
			document.removeEventListener("selectionchange", change);
		};
	});
	/** 글을 끌어서 고른 뒤 오른쪽 클릭하면 고른 글 메뉴. 고치는 중(입력칸)이거나 고른 글이 없으면 띄우지 않는다 */
	function onSelectMenu(e: MouseEvent) {
		if (disabled) return;
		if ((e.target as Element | null)?.closest("textarea, input, [contenteditable='true']")) return;
		const p = pickedText();
		if (!p) return;
		e.preventDefault();
		if (coarse) return menuAfterSettle();
		selMenu = { x: e.clientX, y: e.clientY, text: p.text, messageId: p.messageId };
	}
	$effect(() => {
		const el = scroller;
		if (!el) return;
		el.addEventListener("contextmenu", onSelectMenu);
		return () => el.removeEventListener("contextmenu", onSelectMenu);
	});

	function speakerOf(messageId: string): CardInput {
		const msg = session?.messages.find((x) => x.id === messageId);
		return { text: "", speaker: msg?.name ?? (msg?.role === "assistant" ? characterName : undefined), time: msg?.createdAt };
	}
	function cardSelection() {
		const m = selMenu;
		if (m) oncard({ ...speakerOf(m.messageId), text: m.text });
	}
	async function bookmarkSelection() {
		const m = selMenu; // 메뉴가 닫히며 selMenu가 비워지므로 먼저 잡아 둠
		if (!m || !conv) return;
		const msg = session?.messages.find((x) => x.id === m.messageId);
		await bookmarks.add(conv, m.messageId, m.text, msg ? (metaOf(msg).swipeId ?? 0) : 0);
		window.getSelection()?.removeAllRanges();
	}
</script>

<ContextMenu
	at={selMenu}
	label="고른 글"
	items={[
		{
			label: "복사",
			icon: Copy,
			onselect: () =>
				navigator.clipboard.writeText(selMenu?.text ?? "").then(
					() => toasts.success("복사함"),
					() => toasts.error("복사 못 함"),
				),
		},
		{ label: "북마크", icon: Bookmark, onselect: bookmarkSelection },
		{ label: "이미지 카드로 만들기", icon: IdCard, onselect: cardSelection },
	]}
	onclose={() => (selMenu = null)}
/>
