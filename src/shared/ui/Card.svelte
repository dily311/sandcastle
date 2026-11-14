<script lang="ts">
	import type { Snippet } from "svelte";
	interface Props {
		/** 카드 머리 제목 */
		title?: string;
		description?: string;
		/** 머리 오른쪽 동작 */
		actions?: Snippet;
		padding?: "none" | "sm" | "md";
		children: Snippet;
	}
	let { title, description, actions, padding = "md", children }: Props = $props();
</script>

<section class="card pad-{padding}">
	{#if title || actions}
		<header>
			<div>
				{#if title}<h3>{title}</h3>{/if}
				{#if description}<p class="desc">{description}</p>{/if}
			</div>
			{#if actions}<div class="actions">{@render actions()}</div>{/if}
		</header>
	{/if}
	{@render children()}
</section>

<style>
	.card {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-sm);
		min-width: 0;
	}
	.pad-sm {
		padding: var(--space-3);
	}
	.pad-md {
		padding: var(--space-5);
	}
	header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-4);
		margin-bottom: var(--space-4);
	}
	.desc {
		margin-top: 2px;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.actions {
		display: flex;
		gap: var(--space-2);
		flex: none;
	}
</style>
