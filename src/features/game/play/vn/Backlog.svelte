<script lang="ts">
	import { onMount } from "svelte";
	import { X } from "@lucide/svelte";
	import type { TranscriptEntry_Serialize as Entry } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { gameStore } from "$features/game/runtime/gameStore.svelte";
	import { formatGameTime, isNarration } from "./playUtils";

	// 지난 대사: 씬 대본과 대화를 읽을 수 있는 글로 (오래된 것 → 최신)
	type Props = { sessionId: string; onClose: () => void };
	let { sessionId, onClose }: Props = $props();

	let entries = $state<Entry[]>([]);
	let hasMore = $state(false);
	let oldest = $state<number | null>(null);
	let loading = $state(true);
	let box: HTMLDivElement | undefined = $state();

	async function load(before: number | null = null) {
		loading = true;
		try {
			const page = await call(commands.readTranscript(sessionId, before, 60), { source: "game" });
			entries = before === null ? page.entries : [...page.entries, ...entries];
			hasMore = page.hasMore;
			oldest = page.oldestSeq;
		} finally {
			loading = false;
		}
	}
	onMount(async () => {
		await load();
		box?.scrollTo({ top: box.scrollHeight });
	});

	const sceneName = (id: string | null) => (gameStore.view?.scenes?.find((s: { id: string }) => s.id === id) as { name?: string } | undefined)?.name;
	function speaker(id: string | null | undefined) {
		if (!id) return null;
		if (id === "player") return "나";
		return gameStore.view?.characters.find((c: { id: string }) => c.id === id)?.name ?? id;
	}
</script>

<aside class="g-sheet backlog" aria-label="지난 대사">
	<div class="g-sheet-head">
		<div><strong>지난 대사</strong><small>씬 대본과 대화 기록</small></div>
		<button class="g-icon" onclick={onClose} aria-label="닫기 (Esc)"><X /></button>
	</div>
	<div class="g-sheet-body" bind:this={box}>
		{#if hasMore}<button class="g-btn more" disabled={loading} onclick={() => load(oldest)}>더 이전 기록</button>{/if}
		{#each entries as e (e.seq)}
			<section class="entry">
				<p class="meta">{[formatGameTime(e.gameTime), e.kind === "sceneScript" ? sceneName(e.sceneId) : "대화"].filter(Boolean).join(" · ")}</p>
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
			<p class="empty">{loading ? "불러오는 중…" : "아직 기록 없음"}</p>
		{/each}
	</div>
</aside>

<style>
	.backlog {
		z-index: 90;
		width: min(460px, calc(100% - 24px));
	}
	.more {
		width: 100%;
		margin-bottom: 10px;
	}
	.entry + .entry {
		margin-top: 14px;
		padding-top: 12px;
		border-top: 1px solid var(--game-border-subtle);
	}
	.meta {
		margin: 0 0 6px;
		color: var(--game-text-dim);
		font-size: 11px;
	}
	.line {
		display: flex;
		flex-direction: column;
		margin: 0 0 8px;
		color: var(--game-text-main);
		font-size: 14px;
		line-height: 1.65;
		white-space: pre-wrap;
	}
	.line b {
		color: var(--game-accent);
		font-size: 12px;
	}
	.line.me b {
		color: var(--game-text-secondary);
	}
	.line.narr {
		color: var(--game-text-muted);
		font-style: italic;
	}
	.empty {
		color: var(--game-text-muted);
		font-size: 13px;
	}
</style>
