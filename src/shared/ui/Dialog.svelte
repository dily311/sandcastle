<script lang="ts">
	import { X } from "@lucide/svelte";
	import type { Snippet } from "svelte";
	import IconButton from "./IconButton.svelte";

	interface Props {
		open?: boolean;
		title: string;
		description?: string;
		size?: "sm" | "md" | "lg" | "xl";
		/** 바닥 버튼 영역 */
		footer?: Snippet;
		onclose?: () => void;
		children: Snippet;
	}
	let { open = $bindable(false), title, description, size = "md", footer, onclose, children }: Props = $props();

	let dialog: HTMLDialogElement | undefined = $state();
	const titleId = `dlg-${Math.random().toString(36).slice(2, 9)}`;

	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) dialog.showModal();
		if (!open && dialog.open) dialog.close();
	});

	function close() {
		open = false;
		onclose?.();
	}
</script>

<dialog
	bind:this={dialog}
	class="dialog {size}"
	aria-labelledby={titleId}
	onclose={close}
	onclick={(e) => e.target === dialog && close()}
>
	<div class="panel">
		<header>
			<div>
				<h2 id={titleId}>{title}</h2>
				{#if description}<p class="desc">{description}</p>{/if}
			</div>
			<IconButton label="닫기" size="sm" onclick={close}><X /></IconButton>
		</header>
		<div class="body">{@render children()}</div>
		{#if footer}<footer>{@render footer()}</footer>{/if}
	</div>
</dialog>

<style>
	.dialog {
		width: min(calc(100vw - 32px), 480px);
		max-height: min(calc(100vh - 48px), 760px);
		padding: 0;
		/* 스크롤은 본문(.body) 하나만: 창 자체가 테두리 몇 px 때문에 또 스크롤되지 않게 */
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-xl);
		background: var(--surface);
		color: var(--text);
		box-shadow: var(--shadow-lg);
	}
	.sm {
		width: min(calc(100vw - 32px), 380px);
	}
	.lg {
		width: min(calc(100vw - 32px), 720px);
	}
	.xl {
		width: min(calc(100vw - 32px), 1020px);
	}
	.dialog::backdrop {
		background: var(--overlay);
		backdrop-filter: blur(2px);
	}
	.dialog[open] {
		animation: pop var(--duration) var(--ease);
	}
	@keyframes pop {
		from {
			opacity: 0;
			transform: translateY(6px) scale(0.98);
		}
	}
	.panel {
		display: flex;
		flex-direction: column;
		max-height: min(calc(100vh - 50px), 758px);
	}
	header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-4);
		padding: var(--space-5) var(--space-5) var(--space-3);
	}
	h2 {
		font-size: var(--text-lg);
	}
	.desc {
		margin-top: 4px;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.body {
		padding: 0 var(--space-5) var(--space-5);
		min-height: 0;
		overflow: auto;
	}
	footer {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
		padding: var(--space-4) var(--space-5);
		border-top: 1px solid var(--border);
		background: var(--surface-2);
		border-radius: 0 0 var(--radius-xl) var(--radius-xl);
	}
</style>
