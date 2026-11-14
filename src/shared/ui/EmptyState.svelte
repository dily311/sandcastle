<script lang="ts">
	import type { Snippet } from "svelte";
	interface Props {
		title: string;
		description?: string;
		icon?: Snippet;
		action?: Snippet;
	}
	let { title, description, icon, action }: Props = $props();
</script>

<div class="empty">
	{#if icon}<div class="icon">{@render icon()}</div>{/if}
	<h3>{title}</h3>
	{#if description}<p>{description}</p>{/if}
	{#if action}<div class="action">{@render action()}</div>{/if}
</div>

<style>
	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: var(--space-2);
		padding: var(--space-10) var(--space-6);
		color: var(--text-muted);
	}
	.icon {
		display: grid;
		place-items: center;
		width: 52px;
		height: 52px;
		margin-bottom: var(--space-2);
		border-radius: var(--radius-lg);
		background: var(--accent-soft);
		color: var(--accent-text);
	}
	.icon :global(svg) {
		width: 24px;
		height: 24px;
	}
	h3 {
		color: var(--text);
	}
	p {
		max-width: 36ch;
		font-size: var(--text-sm);
	}
	.action {
		margin-top: var(--space-3);
	}
</style>
