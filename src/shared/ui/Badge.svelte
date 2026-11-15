<script lang="ts">
	import type { Snippet } from "svelte";
	type Tone = "neutral" | "accent" | "success" | "warning" | "danger" | "info";
	let { tone = "neutral", children }: { tone?: Tone; children: Snippet } = $props();
</script>

<span class="badge {tone}">{@render children()}</span>

<style>
	.badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 4px;
		height: 20px;
		padding: 0 var(--space-2);
		border-radius: var(--radius-full);
		font-size: var(--text-xs);
		font-weight: 600;
		text-align: center;
		white-space: nowrap;
	}
	.badge :global(svg) {
		width: 12px;
		height: 12px;
	}
	.neutral {
		background: var(--bg-sunken);
		color: var(--text-muted);
	}
	.accent {
		background: var(--accent-soft);
		color: var(--accent-text);
	}
	.success {
		background: var(--success-soft);
		color: var(--success);
	}
	.warning {
		background: var(--warning-soft);
		color: var(--warning);
	}
	.danger {
		background: var(--danger-soft);
		color: var(--danger);
	}
	.info {
		background: var(--info-soft);
		color: var(--info);
	}
</style>
