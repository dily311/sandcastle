<script lang="ts" generics="T extends string">
	interface Props {
		value: T;
		options: { value: T; label: string }[];
		label: string;
		onchange?: (v: T) => void;
	}
	let { value = $bindable(), options, label, onchange }: Props = $props();

	function select(v: T) {
		value = v;
		onchange?.(v);
	}
	function onkeydown(e: KeyboardEvent) {
		const i = options.findIndex((o) => o.value === value);
		if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
			e.preventDefault();
			const next = options[(i + (e.key === "ArrowRight" ? 1 : options.length - 1)) % options.length];
			select(next.value);
		}
	}
</script>

<div class="segmented" role="radiogroup" aria-label={label} tabindex="-1" {onkeydown}>
	{#each options as o (o.value)}
		<button
			type="button"
			role="radio"
			aria-checked={value === o.value}
			tabindex={value === o.value ? 0 : -1}
			class:selected={value === o.value}
			onclick={() => select(o.value)}
		>
			{o.label}
		</button>
	{/each}
</div>

<style>
	.segmented {
		display: inline-flex;
		padding: 3px;
		gap: 2px;
		background: var(--bg-sunken);
		border-radius: var(--radius-sm);
	}
	button {
		height: calc(var(--control-md) - 6px);
		padding: 0 var(--space-3);
		border: none;
		border-radius: calc(var(--radius-sm) - 2px);
		background: transparent;
		color: var(--text-muted);
		font-weight: 600;
		font-size: var(--text-sm);
		cursor: pointer;
		transition:
			background var(--duration-fast) var(--ease),
			color var(--duration-fast) var(--ease);
	}
	button:hover {
		color: var(--text);
	}
	.selected {
		background: var(--surface);
		color: var(--text);
		box-shadow: var(--shadow-sm);
	}
</style>
