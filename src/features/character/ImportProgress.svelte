<script lang="ts">
	// 여러 채팅 파일 가져오기의 진행 (화면 아래, 가져오는 동안만)
	import { Square } from "@lucide/svelte";
	import { IconButton } from "$shared/ui";
	import { chatImport } from "./chatImport.svelte";
</script>

{#if chatImport.running}
	<div class="import" role="status" aria-live="polite">
		<div class="text">
			<b>대화 가져오는 중 {chatImport.done}/{chatImport.total}</b>
			<span class="file">{chatImport.stopping ? "이 파일까지 하고 멈춤" : chatImport.current}</span>
		</div>
		<progress max={chatImport.total} value={chatImport.done}></progress>
		<IconButton label="멈추기 (지금 파일까지)" size="sm" disabled={chatImport.stopping} onclick={() => chatImport.stop()}><Square /></IconButton>
	</div>
{/if}

<style>
	.import {
		position: fixed;
		left: 50%;
		bottom: calc(var(--space-6) + env(safe-area-inset-bottom, 0px));
		transform: translateX(-50%);
		z-index: var(--z-toast);
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: var(--space-1) var(--space-3);
		width: min(420px, calc(100vw - var(--space-6)));
		padding: var(--space-3) var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: var(--shadow-md);
	}
	.text {
		display: flex;
		flex-direction: column;
		min-width: 0;
		font-size: var(--text-sm);
	}
	.file {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--text-muted);
		font-size: var(--text-xs);
	}
	progress {
		grid-column: 1;
		width: 100%;
		height: 6px;
		accent-color: var(--accent);
	}
	.import :global(button) {
		grid-row: 1 / span 2;
		grid-column: 2;
	}
</style>
