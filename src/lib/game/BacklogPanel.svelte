<script lang="ts">
	import { X } from "@lucide/svelte";
	import type { LogEntry } from "./stage/GameStageController.svelte";

	interface Props {
		log: LogEntry[];
		onClose: () => void;
	}
	let { log, onClose }: Props = $props();
</script>

<div class="backlog-panel-wrap">
	<button class="closeBtn" onclick={onClose}>
		<X size={16} />
	</button>

	<div class="backlog">
		{#each log as item}
			<div>
				{JSON.stringify(item)}
			</div>
		{/each}
	</div>
</div>

<style>
	.backlog-panel-wrap {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 99;
		width: 100%;
		height: 100%;

		padding: 1rem 0.8rem;
		background: #111111be;
		color: #fff;
	}

	.closeBtn {
		position: absolute;
		top: 0;
		right: 0;
		padding: 1rem;
	}
	.closeBtn:hover {
		background: var(--button-hover-bg);
	}

	.backlog {
		display: flex;
		flex-direction: column;
	}
</style>
