<script lang="ts">
	import { _ } from "svelte-i18n";
	// 빠른 답장 버튼 줄 (5-E-20-5-3·4·5): 입력칸 위. 오른쪽 끝 버튼으로 이 대화에서 세트를 끄고 켠다.
	// 버튼이 많으면 앞의 MAX_CHIPS개만 두고 "모두 보기"로 검색 목록을 펼친다 (오른쪽 패널 "빠른 답장" 탭과 같은 목록)
	import { onMount } from "svelte";
	import { Zap } from "@lucide/svelte";
	import { ContextMenu, type MenuItem } from "$shared/ui";
	import type { Conversation } from "$features/character/library.svelte";
	import type { ChatSession } from "$features/character/session.svelte";
	import { nav } from "$app/nav.svelte";
	import { buttonsOf, offIds, quickReplies, runQuickReply, setsFor, setsInScope, toggleForConv, type QuickButton } from "./quickReply.svelte";
	import QuickReplyList from "./QuickReplyList.svelte";
	import { slashRuns } from "./running.svelte";
	import { Square } from "@lucide/svelte";

	const MAX_CHIPS = 6;

	let { session, conv, getDraft, setDraft }: { session: ChatSession; conv: Conversation | undefined; getDraft: () => string; setDraft: (v: string) => void } = $props();

	onMount(() => void quickReplies.load());
	const inScope = $derived(setsInScope(conv).filter((s) => s.enabled));
	const buttons = $derived(buttonsOf(setsFor(conv)));
	const shown = $derived(buttons.length > MAX_CHIPS ? buttons.slice(0, MAX_CHIPS) : buttons);
	/** 이 대화에서 스크립트가 도는 중 (5-E-20-6-8) */
	const scriptRunning = $derived(!!conv && slashRuns.isRunning(conv.id));
	let showAll = $state(false);
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
		...inScope.map((s) => ({ id: s.id, label: s.name || $_("extraTools.qrUnnamedSet"), checked: !offIds(conv).includes(s.id), onselect: () => conv && void toggleForConv(conv, s.id) })),
		{
			label: $_("extraTools.qrHideBar"),
			checked: quickReplies.barHidden,
			separator: inScope.length > 0,
			onselect: () => {
				const next = !quickReplies.barHidden;
				quickReplies.setBarHidden(next);
				if (next) showAll = false;
			},
		},
		{ label: $_("extraTools.qrSettings"), onselect: () => nav.go({ page: "settings", tab: "quickreply" }) },
	]);
</script>

{#if showAll && !quickReplies.barHidden}
	<div class="all" role="dialog" aria-label={$_("extraTools.qrShowAllAria")} tabindex="-1" onkeydown={(e) => e.key === "Escape" && (showAll = false)}>
		<QuickReplyList {session} {conv} {getDraft} {setDraft} autofocus onran={() => (showAll = false)} />
	</div>
{/if}
{#if inScope.length}
	<div class="qr" class:collapsed={quickReplies.barHidden} role="toolbar" aria-label={$_("extraTools.qrToolbarAria")}>
		<div class="list">
			{#if scriptRunning}
				<button type="button" class="chip stop" title={$_("extraTools.qrStopTitle")} onclick={() => conv && slashRuns.stop(conv.id, session)}><Square />{$_("extraTools.qrRunningStop")}</button>
			{/if}
			{#if !quickReplies.barHidden}
				{#each shown as b (b.q.id)}
					<button type="button" class="chip" disabled={running || session.busy || scriptRunning} title={b.q.message} onclick={() => press(b)}>{b.q.label.trim() || b.q.message.slice(0, 16)}</button>
				{/each}
				{#if buttons.length > MAX_CHIPS || showAll}
					<button type="button" class="chip more" aria-expanded={showAll} onclick={() => (showAll = !showAll)}>{showAll ? $_("common.close") : $_("extraTools.qrShowAllBtn", { values: { count: buttons.length } })}</button>
				{/if}
			{/if}
		</div>
		<button type="button" class="pick" class:muted={quickReplies.barHidden} aria-label={$_("extraTools.qrSetMenuAria")} title={quickReplies.barHidden ? $_("extraTools.qrSetMenuHidden") : $_("extraTools.qrSetMenuAria")} onclick={openMenu}><Zap /></button>
	</div>
{/if}
{#if menu}
	<ContextMenu at={{ x: menu.x, y: menu.y, align: "end" }} items={menuItems} label={$_("extraTools.qrSetTitle")} onclose={() => (menu = null)} />
{/if}

<style>
	.qr {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-2);
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
	.chip.stop {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		border-color: var(--accent);
		color: var(--accent-text, var(--accent));
	}
	.chip.stop :global(svg) {
		width: 12px;
		height: 12px;
	}
	.chip.more {
		border-style: dashed;
		color: var(--text-muted);
	}
	.all {
		max-height: min(50vh, 420px);
		overflow-y: auto;
		margin: 0 var(--space-1) var(--space-2);
		padding: var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-md, none);
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
	.qr.collapsed {
		padding-top: 0;
		padding-bottom: 0;
	}
	.pick.muted {
		opacity: 0.55;
	}
	.pick:hover {
		color: var(--text);
		opacity: 1;
	}
</style>
