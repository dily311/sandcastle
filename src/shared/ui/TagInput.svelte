<script lang="ts">
	import { X } from "@lucide/svelte";

	interface Props {
		value: string[];
		placeholder?: string;
		label?: string;
	}
	let { value = $bindable([]), placeholder = "태그 입력", label = "태그 추가" }: Props = $props();
	let draft = $state("");

	/** 입력 중인 글자를 태그로 (저장 직전에도 부름) */
	export function commit() {
		const t = draft.trim().replace(/,$/, "").trim();
		if (t && !value.includes(t)) value = [...value, t];
		draft = "";
	}
	function onkeydown(e: KeyboardEvent) {
		if ((e.key === "Enter" || e.key === ",") && !e.isComposing) {
			e.preventDefault();
			commit();
		} else if (e.key === "Backspace" && !draft && value.length) value = value.slice(0, -1);
	}
</script>

<div class="tags">
	{#each value as t (t)}
		<span class="tag">{t}<button type="button" aria-label="{t} 태그 지우기" onclick={() => (value = value.filter((x) => x !== t))}><X /></button></span>
	{/each}
	<input bind:value={draft} {onkeydown} onblur={commit} placeholder={value.length ? "" : placeholder} aria-label={label} />
</div>

<style>
	.tags {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		min-height: var(--control-md);
		padding: 4px var(--space-2);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		background: var(--surface);
	}
	.tags:focus-within {
		border-color: var(--accent);
	}
	.tag {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		height: 24px;
		padding: 0 4px 0 var(--space-2);
		border-radius: 999px;
		background: var(--surface-active);
		font-size: var(--text-sm);
	}
	.tag button {
		display: grid;
		place-items: center;
		width: 18px;
		height: 18px;
		padding: 0;
		border: none;
		border-radius: 50%;
		background: none;
		color: var(--text-faint);
		cursor: pointer;
	}
	.tag button:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.tag button :global(svg) {
		width: 12px;
		height: 12px;
	}
	input {
		flex: 1;
		min-width: 80px;
		height: 26px;
		border: none;
		outline: none;
		background: transparent;
		color: var(--text);
		font: inherit;
	}
</style>
