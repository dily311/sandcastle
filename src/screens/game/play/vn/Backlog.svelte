<script lang="ts">
	import Icon from "$/components/ui/Icon.svelte";
	import { gameEngineCommands } from "$/ipc/commands";
	import type { GameJournal } from "$/lib/type/game";
	import { onMount } from "svelte";

	type Props = { sessionId: string; onClose: () => void };
	let { sessionId, onClose }: Props = $props();

	let journals = $state<GameJournal[]>([]);

	async function loadJournals() {
		const history = await gameEngineCommands.readJournal({
			sessionId,
			toSeq: 100, // 최근 100개 기준 (필요에 따라 조절)
		});
		journals = history;
	}
	onMount(() => {
		loadJournals();
	});
</script>

<div class="backlog">
	<button class="closeBtn" onclick={onClose}>
		<Icon name="x" size={24} />
	</button>
	<section class="log-section">
		<div class="log-box">
			{#each journals as journal}
				<div class="log-item">
					<span class="time">[{journal.gameTime}분]</span>
					<span class="kind">({journal.kind})</span>
					<span class="payload">{JSON.stringify(journal.payloadJson)}</span>
				</div>
			{:else}
				<div class="empty-log">기록된 저널이 없습니다.</div>
			{/each}
		</div>
	</section>
</div>

<style>
	.backlog {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 99;
		width: 100%;
		height: 100%;

		padding: 1rem 0.8rem;
		background: #111111e7;
		color: #fff;
		overflow: hidden;
	}

	.closeBtn {
		position: absolute;
		top: 0;
		right: 0;
		padding: 1rem;
		color: #fff;
	}
	.closeBtn:hover {
		background: var(--button-hover-bg);
	}
	.log-section {
		width: 100%;
		height: 100%;
		border: 1px solid var(--border-color);
		display: flex;
		flex-direction: column;
		padding: 1rem;
		overflow: hidden scroll;
	}

	.log-box {
		flex: 1;
		padding: 0.8rem;

		overflow: hidden auto;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		font-size: 0.875rem;
	}

	.log-item {
		font-family: monospace;
		white-space: break-spaces;
		line-height: 1.4;
	}
	.log-item .time {
		color: var(--color-blue);
		margin-right: 0.4rem;
	}
	.log-item .kind {
		color: var(--color-purple);
		margin-right: 0.4rem;
	}
	.empty-log {
		text-align: center;
		color: #4b5563;
		margin: auto;
		font-size: 0.85rem;
	}
</style>
