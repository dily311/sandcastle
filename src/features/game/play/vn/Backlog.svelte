<script lang="ts">
	// 지난 대사 창 (무대 위에 뜸). 지금 씬이 열려 있으면 그 씬의 대사만, NPC와 대화 중이면 그 대화의 대사만 보여준다.
	// 둘 다 아니면 최근 기록. 백엔드: read_current_transcript
	import { X } from "@lucide/svelte";
	import type { TranscriptEntry_Serialize as Entry } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { gameStore } from "../../runtime/gameStore.svelte";
	import { runtimeStore } from "../../runtime/runtimeStore.svelte";
	import { formatGameTime, isNarration } from "./playUtils";

	let { onclose }: { onclose: () => void } = $props();

	let entries = $state<Entry[]>([]);
	let hasMore = $state(false);
	let loading = $state(true);
	let box: HTMLDivElement | undefined = $state();

	const sceneId = $derived(runtimeStore.currentSceneId ?? null);
	const npcId = $derived(runtimeStore.activeNpcId ?? null);
	const title = $derived.by(() => {
		if (sceneId) return `이 장면의 대사 · ${gameStore.scenes?.find((s) => s.id === sceneId)?.name ?? "지금 장면"}`;
		if (npcId) return `${speaker(npcId) ?? "상대"}와의 대화`;
		return "최근 대사";
	});

	$effect(() => {
		// 열린 동안 턴이 끝나면 새로
		void runtimeStore.value;
		loading = true;
		void call(commands.readCurrentTranscript(200), { source: "game", silent: true })
			.then((page) => {
				entries = page.entries;
				hasMore = page.hasMore;
			})
			.catch(() => (entries = []))
			.finally(() => {
				loading = false;
				requestAnimationFrame(() => box?.scrollTo({ top: box.scrollHeight }));
			});
	});

	function speaker(id: string | null | undefined) {
		if (!id) return null;
		if (id === "player") return "나";
		return gameStore.view?.characters.find((c) => c.id === id)?.name ?? id;
	}
</script>

<svelte:window onkeydown={(e) => (e.key === "Escape" || e.key === "l") && (e.preventDefault(), e.stopPropagation(), onclose())} />

<div class="backdrop" role="presentation" onclick={onclose}></div>
<div class="backlog" role="dialog" aria-modal="true" aria-label={title}>
	<header>
		<h3>{title}</h3>
		<button class="close" type="button" aria-label="닫기 (Esc)" title="닫기 (Esc)" onclick={onclose}><X /></button>
	</header>
	<div class="body" bind:this={box}>
		{#if hasMore}<p class="more">이보다 앞의 대사는 생략됨</p>{/if}
		{#each entries as e (e.seq)}
			<section class="entry">
				<p class="meta">{formatGameTime(e.gameTime)}</p>
				{#each e.lines as l, i (i)}
					{#if l.text?.trim()}
						{@const who = isNarration(l as never) ? null : speaker(l.speaker)}
						<p class="line" class:me={l.speaker === "player"} class:narr={!who}>
							{#if who}<b>{who}</b>{/if}<span>{l.text}</span>
						</p>
					{/if}
				{/each}
			</section>
		{:else}
			<p class="none">{loading ? "불러오는 중…" : "아직 대사가 없음"}</p>
		{/each}
	</div>
</div>

<style>
	.backdrop {
		position: absolute;
		inset: 0;
		z-index: 30;
		background: rgb(0 0 0 / 0.45);
	}
	.backlog {
		position: absolute;
		z-index: 40;
		inset: 6% 8%;
		display: flex;
		flex-direction: column;
		border: 1px solid var(--game-border);
		border-radius: 16px;
		background: var(--game-surface, var(--surface));
		color: var(--game-text-main, var(--text));
		box-shadow: var(--shadow-lg);
		overflow: hidden;
	}
	header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px 16px;
		border-bottom: 1px solid var(--game-border);
	}
	h3 {
		flex: 1;
		margin: 0;
		font-size: 15px;
	}
	.close {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		border: none;
		border-radius: 8px;
		background: transparent;
		color: inherit;
		cursor: pointer;
	}
	.close:hover {
		background: var(--surface-hover);
	}
	.close :global(svg) {
		width: 18px;
		height: 18px;
	}
	.body {
		flex: 1;
		overflow: auto;
		padding: 8px 20px 20px;
	}
	.entry {
		padding: 10px 0;
		border-bottom: 1px solid var(--game-border);
	}
	.meta,
	.more {
		margin: 0 0 4px;
		font-size: 12px;
		color: var(--game-text-muted, var(--text-faint));
	}
	.line {
		display: flex;
		flex-direction: column;
		margin: 0 0 6px;
		font-size: 15px;
		line-height: 1.6;
	}
	.line b {
		font-size: 12px;
		color: var(--accent-text);
	}
	.line.me b {
		color: var(--game-text-muted, var(--text-muted));
	}
	.line span {
		white-space: pre-wrap;
	}
	.line.narr span {
		color: var(--game-text-secondary, var(--text-muted));
		font-style: italic;
	}
	.none {
		margin: 16px 0;
		color: var(--game-text-muted, var(--text-faint));
	}
	@media (max-width: 620px) {
		.backlog {
			inset: 0;
			border-radius: 0;
		}
	}
</style>
