<script lang="ts" module>
	export interface CardInput {
		text: string;
		speaker?: string;
		time?: number;
	}
</script>

<script lang="ts">
	// 글을 끌어서 고르면: 북마크 / 이미지 카드 플로팅 버튼 (대화 화면·읽기 화면이 같이 씀, 5-E-6-2)
	// 우클릭은 가로채지 않고 브라우저/OS 기본 컨텍스트 메뉴(복사·번역 등)가 그대로 열리게 둔다.
	import { Bookmark, IdCard } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
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

	let bar = $state<{ x: number; y: number; text: string; messageId: string } | null>(null);
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

	const coarse = typeof matchMedia !== "undefined" && matchMedia("(pointer: coarse)").matches;
	let dragging = false;
	let selTimer: ReturnType<typeof setTimeout> | undefined;

	function showForSelection() {
		if (dragging || disabled) return;
		const p = pickedText();
		if (!p || (p.rect.width === 0 && p.rect.height === 0)) {
			bar = null;
			return;
		}
		const x = Math.min(Math.max(p.rect.left + p.rect.width / 2, 72), window.innerWidth - 72);
		const above = p.rect.top > 56;
		const y = above ? p.rect.top - 8 : p.rect.bottom + 8;
		bar = { x, y: above ? y : y, text: p.text, messageId: p.messageId };
	}

	function scheduleShow(delay: number) {
		clearTimeout(selTimer);
		selTimer = setTimeout(showForSelection, delay);
	}

	$effect(() => {
		const el = scroller;
		if (!el) return;
		const down = (e: PointerEvent) => {
			if ((e.target as Element | null)?.closest(".sel-bar")) return;
			dragging = true;
			clearTimeout(selTimer);
			if (e.button === 0) bar = null;
		};
		const up = () => {
			if (!dragging) return;
			dragging = false;
			scheduleShow(coarse ? 450 : 60);
		};
		const change = () => {
			if (dragging) return;
			const sel = window.getSelection();
			if (!sel || sel.isCollapsed) {
				bar = null;
				return;
			}
			if (coarse) scheduleShow(450);
		};
		// 우클릭 시 기본 시스템 컨텍스트 메뉴와 겹치지 않게 플로팅 바만 숨김 (preventDefault 하지 않음)
		const onContext = () => {
			bar = null;
		};
		const onScroll = () => {
			bar = null;
		};
		document.addEventListener("pointerdown", down, { passive: true });
		document.addEventListener("pointerup", up, { passive: true });
		document.addEventListener("pointercancel", up, { passive: true });
		document.addEventListener("selectionchange", change);
		el.addEventListener("contextmenu", onContext);
		el.addEventListener("scroll", onScroll, { passive: true });
		return () => {
			clearTimeout(selTimer);
			document.removeEventListener("pointerdown", down);
			document.removeEventListener("pointerup", up);
			document.removeEventListener("pointercancel", up);
			document.removeEventListener("selectionchange", change);
			el.removeEventListener("contextmenu", onContext);
			el.removeEventListener("scroll", onScroll);
		};
	});

	function speakerOf(messageId: string): CardInput {
		const msg = session?.messages.find((x) => x.id === messageId);
		return { text: "", speaker: msg?.name ?? (msg?.role === "assistant" ? characterName : undefined), time: msg?.createdAt };
	}
	function cardSelection() {
		const m = bar;
		bar = null;
		if (m) oncard({ ...speakerOf(m.messageId), text: m.text });
	}
	async function bookmarkSelection() {
		const m = bar;
		bar = null;
		if (!m || !conv) return;
		const msg = session?.messages.find((x) => x.id === m.messageId);
		await bookmarks.add(conv, m.messageId, m.text, msg ? (metaOf(msg).swipeId ?? 0) : 0);
		window.getSelection()?.removeAllRanges();
	}
</script>

{#if bar && !disabled}
	<div class="sel-bar" role="toolbar" aria-label={$_("textCard.selToolbarAria")} style="left: {bar.x}px; top: {bar.y}px;">
		<button type="button" onmousedown={(e) => e.preventDefault()} onclick={() => void bookmarkSelection()}>
			<Bookmark size={13} /><span>{$_("textCard.selBookmark")}</span>
		</button>
		<button type="button" onmousedown={(e) => e.preventDefault()} onclick={cardSelection}>
			<IdCard size={13} /><span>{$_("textCard.selCard")}</span>
		</button>
	</div>
{/if}

<style>
	.sel-bar {
		position: fixed;
		transform: translate(-50%, -100%);
		z-index: var(--z-popover, 600);
		display: inline-flex;
		align-items: center;
		gap: 2px;
		padding: 3px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: var(--shadow-md, 0 4px 16px rgba(0, 0, 0, 0.2));
	}
	.sel-bar button {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		padding: 4px 8px;
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
		white-space: nowrap;
	}
	.sel-bar button:hover {
		background: var(--surface-hover);
	}
</style>
