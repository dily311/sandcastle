<script lang="ts">
	// 게임 패널 '지난 대사': 씬 대본과 대화를 읽을 수 있는 글로 (오래된 것 → 최신, 한 턴 끝날 때마다 새로)
	import type { TranscriptEntry_Serialize as Entry } from "$ipc/bindings";
	import { Button } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { gameStore } from "../runtime/gameStore.svelte";
	import { runtimeStore } from "../runtime/runtimeStore.svelte";
	import { formatGameTime, isNarration } from "./vn/playUtils";

	let { sessionId }: { sessionId: string } = $props();

	let entries = $state<Entry[]>([]);
	let hasMore = $state(false);
	let oldest = $state<number | null>(null);
	let loading = $state(false);
	let box: HTMLDivElement | undefined = $state();

	async function load(before: number | null = null) {
		loading = true;
		try {
			const page = await call(commands.readTranscript(sessionId, before, 60), { source: "game", silent: true });
			entries = before === null ? page.entries : [...page.entries, ...entries];
			hasMore = page.hasMore;
			oldest = page.oldestSeq;
		} catch {
			/* 다음에 다시 */
		} finally {
			loading = false;
		}
	}
	$effect(() => {
		void sessionId;
		void runtimeStore.value;
		void load().then(() => box?.closest(".panel")?.scrollTo({ top: 1e9 }));
	});

	const sceneName = (id: string | null) => (gameStore.view?.scenes?.find((s: { id: string }) => s.id === id) as { name?: string } | undefined)?.name;
	function speaker(id: string | null | undefined) {
		if (!id) return null;
		if (id === "player") return "나";
		return gameStore.view?.characters.find((c: { id: string }) => c.id === id)?.name ?? id;
	}
</script>

<div class="panel-body" bind:this={box}>
	<section>
		<h4>지난 대사 <span class="faint">씬 대본과 대화 기록</span></h4>
		{#if hasMore}<Button size="sm" variant="ghost" loading={loading} onclick={() => load(oldest)}>더 이전 기록</Button>{/if}
	</section>
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
		<p class="none">{loading ? "불러오는 중…" : "아직 기록 없음"}</p>
	{/each}
</div>

<style>
	.entry {
		gap: 4px !important;
		padding-top: var(--space-2);
		border-top: 1px solid var(--border);
	}
	.meta {
		margin: 0 0 2px;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.line {
		margin: 0;
		display: flex;
		flex-direction: column;
		font-size: var(--text-sm);
		line-height: 1.55;
	}
	.line b {
		font-size: var(--text-xs);
		color: var(--accent-text);
	}
	.line.me b {
		color: var(--text-muted);
	}
	.line span {
		white-space: pre-wrap;
	}
	.line.narr span {
		color: var(--text-muted);
		font-style: italic;
	}
	.none {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
</style>
