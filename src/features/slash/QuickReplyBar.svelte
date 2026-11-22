<script lang="ts">
	// 빠른 답장 버튼 줄 (5-E-20-5-3·4): 입력칸 위. 오른쪽 끝 버튼으로 이 대화에서 세트를 끄고 켠다
	import { onMount } from "svelte";
	import { Zap } from "@lucide/svelte";
	import { ContextMenu, type MenuItem } from "$shared/ui";
	import type { Conversation } from "$features/character/library.svelte";
	import type { ChatSession } from "$features/character/session.svelte";
	import { nav } from "$app/nav.svelte";
	import { buttonsOf, offIds, quickReplies, runQuickReply, setsFor, setsInScope, toggleForConv, type QuickButton } from "./quickReply.svelte";

	let { session, conv, getDraft, setDraft }: { session: ChatSession; conv: Conversation | undefined; getDraft: () => string; setDraft: (v: string) => void } = $props();

	onMount(() => void quickReplies.load());
	const inScope = $derived(setsInScope(conv).filter((s) => s.enabled));
	const buttons = $derived(buttonsOf(setsFor(conv)));
	let running = $state(false);
	let menu = $state<{ x: number; y: number } | null>(null);

	async function press(b: QuickButton) {
		if (running || session.busy) return;
		running = true;
		try {
			await runQuickReply(b.q, b.set, session, { get: getDraft, set: setDraft });
		} finally {
			running = false;
		}
	}
	function openMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.right, y: r.top - 4 };
	}
	const menuItems = $derived<MenuItem[]>([
		...inScope.map((s) => ({ id: s.id, label: s.name || "이름 없는 세트", checked: !offIds(conv).includes(s.id), onselect: () => conv && void toggleForConv(conv, s.id) })),
		{ label: "빠른 답장 설정", separator: inScope.length > 0, onselect: () => nav.go({ page: "settings", tab: "quickreply" }) },
	]);
</script>

{#if inScope.length}
	<div class="qr" role="toolbar" aria-label="빠른 답장">
		<div class="list">
			{#each buttons as b (b.q.id)}
				<button type="button" class="chip" disabled={running || session.busy} title={b.q.message} onclick={() => press(b)}>{b.q.label.trim() || b.q.message.slice(0, 16)}</button>
			{/each}
		</div>
		<button type="button" class="pick" aria-label="이 대화의 빠른 답장 세트" title="이 대화의 빠른 답장 세트" onclick={openMenu}><Zap /></button>
	</div>
{/if}
{#if menu}
	<ContextMenu at={{ x: menu.x, y: menu.y, align: "end" }} items={menuItems} label="빠른 답장 세트" onclose={() => (menu = null)} />
{/if}

<style>
	.qr {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 0 var(--space-1) var(--space-2);
	}
	.list {
		flex: 1;
		display: flex;
		gap: var(--space-2);
		overflow-x: auto;
		scrollbar-width: thin;
	}
	.chip {
		flex: 0 0 auto;
		max-width: 14em;
		padding: 4px 12px;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: var(--surface);
		color: var(--text);
		font-size: var(--text-sm);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		cursor: pointer;
	}
	.chip:hover:not(:disabled) {
		border-color: var(--accent);
	}
	.chip:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.pick {
		display: grid;
		place-items: center;
		padding: 4px;
		border: none;
		background: none;
		color: var(--text-faint);
		cursor: pointer;
	}
	.pick :global(svg) {
		width: 16px;
		height: 16px;
	}
</style>
