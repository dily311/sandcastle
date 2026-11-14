<script lang="ts">
	import { ACCENTS } from "./accents";

	/** 빈 문자열 = 자동 (이름에서 색을 정함) */
	let { value = $bindable(""), label = "색" }: { value?: string; label?: string } = $props();
</script>

<div class="colors" role="radiogroup" aria-label={label}>
	<button type="button" role="radio" aria-checked={!value} class="auto" class:on={!value} title="이름에서 자동" onclick={() => (value = "")}>자동</button>
	{#each ACCENTS as a (a.id)}
		<button type="button" role="radio" aria-checked={value === a.light} aria-label={a.label} title={a.label} class:on={value === a.light} style:background={a.light} onclick={() => (value = a.light)}></button>
	{/each}
</div>

<style>
	.colors {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}
	button {
		width: 26px;
		height: 26px;
		border-radius: 50%;
		border: 2px solid var(--bg-sunken);
		box-shadow: 0 0 0 1px var(--border-strong);
		cursor: pointer;
	}
	button.on {
		box-shadow: 0 0 0 2px var(--text);
	}
	.auto {
		width: auto;
		padding: 0 var(--space-2);
		border-radius: 999px;
		background: var(--surface);
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
	}
</style>
