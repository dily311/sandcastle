<script lang="ts">
import { cn } from "$/lib/utils/cn";
import type { Snippet } from "svelte";

	interface Props {
		label: string;
		desc?: string;
		class?: string;
		nowrap?: boolean;
		/** 조작 요소를 아래로 내림 (긴 입력칸) */
		stacked?: boolean;
		children?: Snippet;
	}
	let { label, desc, stacked = false, nowrap=false, class: className, children }: Props = $props();
</script>

<div class={cn("row", className)} class:nowrap class:stacked>
	<div class="txt"><b>{label}</b>{#if desc}<span>{desc}</span>{/if}</div>
	{#if children}<div class="ctl">{@render children()}</div>{/if}
</div>

<style>
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-6);
		padding: var(--space-4) 0;
	    flex-wrap: wrap;
		overflow: hidden;
	}
	.row.nowrap {
		flex-wrap: nowrap;
	}
	.row + :global(.row) {
		border-top: 1px solid var(--border);
	}
	.row.stacked {
		flex-direction: column;
		align-items: stretch;
		gap: var(--space-3);
	}
	.txt {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.txt b {
		font-weight: 600;
	}
	.txt span {
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.ctl {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-shrink: 0;
	}
	.stacked .ctl {
		flex-shrink: 1;
		flex-direction: column;
		align-items: stretch;
	}
	.ctl :global(.select) {
		min-width: 160px;
	}

</style>
