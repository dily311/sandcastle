<script lang="ts" generics="T extends string">
	import { ChevronDown } from "@lucide/svelte";
	import type { HTMLSelectAttributes } from "svelte/elements";

	// 제네릭 컴포넌트는 Props 타입을 따로 이름 붙이면 내보내는 타입에서 오류가 나서 바로 적음
	let {
		value = $bindable(),
		options,
		placeholder,
		class: className = "",
		...rest
	}: Omit<HTMLSelectAttributes, "value"> & {
		value?: T;
		/** group이 같은 연속 항목은 <optgroup>으로 묶인다 (group이 없으면 그냥 항목) */
		options: { value: T; label: string; disabled?: boolean; group?: string }[];
		placeholder?: string;
	} = $props();
	const blocks = $derived.by(() => {
		const out: { group?: string; items: typeof options }[] = [];
		for (const o of options) {
			const last = out[out.length - 1];
			if (last && last.group === o.group) last.items.push(o);
			else out.push({ group: o.group, items: [o] });
		}
		return out;
	});
</script>

<div class="select {className}">
	<select bind:value {...rest}>
		{#if placeholder}<option value="" disabled>{placeholder}</option>{/if}
		{#each blocks as b, i (i)}
			{#if b.group}
				<optgroup label={b.group}>
					{#each b.items as o (o.value)}<option value={o.value} disabled={o.disabled}>{o.label}</option>{/each}
				</optgroup>
			{:else}
				{#each b.items as o (o.value)}<option value={o.value} disabled={o.disabled}>{o.label}</option>{/each}
			{/if}
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
