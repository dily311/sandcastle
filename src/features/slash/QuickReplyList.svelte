<script lang="ts">
	// 빠른 답장 검색 목록 (5-E-20-5-5): 버튼 줄의 "모두 보기"와 오른쪽 패널 "빠른 답장" 탭이 같이 쓴다.
	// 이 대화에 해당하는 세트를 묶어 보이고, 세트마다 이 대화에서 끄고 켤 수 있다
	import { onMount } from "svelte";
	import { Search } from "@lucide/svelte";
	import { Switch } from "$shared/ui";
	import type { Conversation } from "$features/character/library.svelte";
	import type { ChatSession } from "$features/character/session.svelte";
	import { buttonsOf, offIds, quickReplies, runQuickReply, setsInScope, toggleForConv, type QuickButton } from "./quickReply.svelte";
	import { slashRuns } from "./running.svelte";

	interface Props {
		session: ChatSession;
		conv: Conversation | undefined;
		getDraft: () => string;
		setDraft: (v: string) => void;
		/** 처음에 검색칸에 커서 */
		autofocus?: boolean;
		/** 하나 누른 뒤 (펼친 목록 닫기 등) */
		onran?: () => void;
	}
	let { session, conv, getDraft, setDraft, autofocus = false, onran }: Props = $props();

	onMount(() => void quickReplies.load());
	let query = $state("");
	let input: HTMLInputElement | undefined = $state();
	$effect(() => {
		if (autofocus) input?.focus();
	});
	const sets = $derived(setsInScope(conv).filter((s) => s.enabled));
	const off = $derived(offIds(conv));
	const groups = $derived(sets.map((set) => ({ set, on: !off.includes(set.id), buttons: buttonsOf([set], query) })).filter((g) => !query.trim() || g.buttons.length));
	const found = $derived(groups.reduce((n, g) => n + (g.on ? g.buttons.length : 0), 0));
	let running = $state(false);

	async function press(b: QuickButton) {
		if (running || session.busy) return;
		running = true;
		try {
			await runQuickReply(b.q, b.set, session, { get: getDraft, set: setDraft });
			onran?.();
		} finally {
			running = false;
		}
	}
	function onkeydown(e: KeyboardEvent) {
		// 검색 중 Enter: 첫 결과 실행
		if (e.key !== "Enter" || e.isComposing) return;
		const first = groups.find((g) => g.on && g.buttons.length)?.buttons[0];
		if (first) {
			e.preventDefault();
			void press(first);
		}
	}
</script>

<div class="qr-list">
	<label class="search">
		<Search />
		<input bind:this={input} bind:value={query} type="search" placeholder="빠른 답장 검색" aria-label="빠른 답장 검색" {onkeydown} />
		{#if query.trim()}<small>{found}개</small>{/if}
	</label>
	{#if !sets.length}
		<p class="none">이 대화에 쓸 수 있는 세트가 없음</p>
	{:else if !groups.length}
		<p class="none">찾는 빠른 답장이 없음</p>
	{/if}
	{#each groups as g (g.set.id)}
		<section class:off={!g.on}>
			<header>
				<b>{g.set.name || "이름 없는 세트"}</b>
				<small>{g.set.scope === "global" ? "전체" : g.set.scope === "character" ? "캐릭터" : "프리셋"}</small>
				<Switch checked={g.on} label="이 대화에서 {g.set.name} 쓰기" hideLabel onchange={() => conv && void toggleForConv(conv, g.set.id)} />
			</header>
			{#if g.on}
				{#each g.buttons as b (b.q.id)}
					<button type="button" class="item" disabled={running || session.busy || (!!conv && slashRuns.isRunning(conv.id))} onclick={() => press(b)}>
						<span class="label">{b.q.label.trim() || b.q.message.slice(0, 16)}</span>
						<span class="msg">{b.q.message}</span>
					</button>
				{/each}
			{/if}
		</section>
	{/each}
</div>

<style>
	.qr-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.search {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 6px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
	}
	.search :global(svg) {
		width: 14px;
		height: 14px;
		color: var(--text-faint);
	}
	.search input {
		flex: 1;
		min-width: 0;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		outline: none;
	}
	.search small {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.none {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
	section {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 2px 0 4px;
		font-size: var(--text-sm);
	}
	header b {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	header small {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	section.off header b {
		color: var(--text-faint);
	}
	.item {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1px;
		width: 100%;
		padding: 6px 8px;
		border: none;
		border-radius: var(--radius-sm);
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.item:hover:not(:disabled) {
		background: var(--surface-hover);
	}
	.item:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.label {
		font-size: var(--text-sm);
		font-weight: 600;
	}
	.msg {
		width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
</style>
