<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		title: string;
		lead?: string;
		/** 제목 오른쪽 버튼 */
		actions?: Snippet;
		children: Snippet;
	}
	let { title, lead, actions, children }: Props = $props();
</script>

<header class="head">
	<div>
		<h1>{title}</h1>
		{#if lead}<p class="lead">{lead}</p>{/if}
	</div>
	{#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>
{@render children()}

<style>
	.head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-4);
		margin-bottom: var(--space-4);
	}
	h1 {
		font-size: var(--text-xl);
	}
	.lead {
		margin-top: var(--space-1);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.actions {
		display: flex;
		gap: var(--space-2);
		flex-shrink: 0;
	}
</style>
