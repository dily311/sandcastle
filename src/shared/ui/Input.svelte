<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLInputAttributes } from "svelte/elements";

	interface Props extends Omit<HTMLInputAttributes, "value"> {
		value?: string | number | null;
		invalid?: boolean;
		/** 입력칸 안 앞쪽 아이콘 */
		leading?: Snippet;
		trailing?: Snippet;
	}
	let { value = $bindable(), invalid = false, leading, trailing, class: className = "", ...rest }: Props = $props();
</script>

<div class="input {className}" class:invalid class:has-leading={!!leading}>
	{#if leading}<span class="adorn">{@render leading()}</span>{/if}
	<input bind:value aria-invalid={invalid || undefined} {...rest} />
	{#if trailing}<span class="adorn">{@render trailing()}</span>{/if}
</div>

<style>
	.input {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		height: var(--control-md);
		padding: 0 var(--space-3);
		background: var(--surface);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		transition:
			border-color var(--duration-fast) var(--ease),
			box-shadow var(--duration-fast) var(--ease);
	}
	.input:hover {
		border-color: color-mix(in oklab, var(--border-strong) 70%, var(--text-faint));
	}
	.input:focus-within {
		border-color: var(--accent);
		box-shadow: 0 0 0 3px var(--focus-ring);
	}
	.invalid,
	.invalid:focus-within {
		border-color: var(--danger);
	}
	input {
		flex: 1;
		min-width: 0;
		height: 100%;
		border: none;
		background: transparent;
		outline: none;
	}
	input::placeholder {
		color: var(--text-faint);
	}
	.adorn {
		display: inline-flex;
		color: var(--text-faint);
	}
	.adorn :global(svg) {
		width: 16px;
		height: 16px;
	}
</style>
