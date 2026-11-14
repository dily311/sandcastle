<script lang="ts" generics="T extends string">
	import { ChevronDown } from "@lucide/svelte";
	import type { HTMLSelectAttributes } from "svelte/elements";

	interface Option {
		value: T;
		label: string;
		disabled?: boolean;
	}
	interface Props extends Omit<HTMLSelectAttributes, "value"> {
		value?: T;
		options: Option[];
		placeholder?: string;
	}
	let { value = $bindable(), options, placeholder, class: className = "", ...rest }: Props = $props();
</script>

<div class="select {className}">
	<select bind:value {...rest}>
		{#if placeholder}<option value="" disabled>{placeholder}</option>{/if}
		{#each options as o (o.value)}
			<option value={o.value} disabled={o.disabled}>{o.label}</option>
		{/each}
	</select>
	<ChevronDown class="chev" aria-hidden="true" />
</div>

<style>
	.select {
		position: relative;
		display: flex;
		align-items: center;
	}
	select {
		appearance: none;
		width: 100%;
		height: var(--control-md);
		padding: 0 calc(var(--space-3) + 20px) 0 var(--space-3);
		background: var(--surface);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		cursor: pointer;
		outline: none;
	}
	select:focus {
		border-color: var(--accent);
		box-shadow: 0 0 0 3px var(--focus-ring);
	}
	.select :global(.chev) {
		position: absolute;
		right: var(--space-3);
		width: 16px;
		height: 16px;
		color: var(--text-faint);
		pointer-events: none;
	}
</style>
