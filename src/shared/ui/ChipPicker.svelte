<script lang="ts">
	// 여러 개 고르기: 고른 것은 칩(× 로 빼기), '+'로 남은 것 중에서 더 넣기 (로어북 고르기와 같은 모양)
	import { Plus, X } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";

	interface Option {
		id: string;
		label: string;
	}
	let {
		options,
		value,
		onchange,
		label,
		empty,
	}: {
		/** 고를 수 있는 것 전부 */
		options: readonly Option[];
		/** 고른 id */
		value: readonly string[];
		onchange: (ids: string[]) => void;
		label: string;
		empty?: string;
	} = $props();

	const picked = $derived(options.filter((o) => value.includes(o.id)));
	let menu = $state<{ x: number; y: number } | null>(null);
	const items = $derived<MenuItem[]>(
		options.filter((o) => !value.includes(o.id)).map((o) => ({ label: o.label, onselect: () => onchange([...value, o.id]) })),
	);
	function openMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.left, y: r.bottom + 4 };
	}
</script>

<div class="picker" role="group" aria-label={label}>
	{#each picked as o (o.id)}
		<span class="chip">
			<span class="name">{o.label}</span>
			<button class="x" aria-label={$_("ui.chipRemove", { values: { label: o.label } })} onclick={() => onchange(value.filter((id) => id !== o.id))}><X /></button>
		</span>
	{:else}
		<span class="empty">{empty ?? $_("ui.chipEmpty")}</span>
	{/each}
	<button class="add" aria-label={$_("ui.chipAdd", { values: { label } })} aria-haspopup="menu" disabled={!items.length} onclick={openMenu}><Plus /></button>
</div>
<ContextMenu at={menu} label={$_("ui.chipAdd", { values: { label } })} {items} onclose={() => (menu = null)} />

<style>
	.picker {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px;
		min-height: 32px;
		padding: 3px;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
	}
	.chip {
		display: inline-flex;
		align-items: center;
		max-width: 100%;
		border-radius: var(--radius-sm);
		background: var(--accent-soft);
		color: var(--accent-text);
		font-size: var(--text-sm);
	}
	.name {
		padding: 2px 2px 2px 8px;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.x {
		display: grid;
		place-items: center;
		width: 20px;
		height: 22px;
		padding: 0;
		border: none;
		background: transparent;
		color: inherit;
		cursor: pointer;
	}
	.x :global(svg),
	.add :global(svg) {
		width: 13px;
		height: 13px;
	}
	.empty {
		padding: 0 6px;
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
	.add {
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		margin-left: auto;
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
	}
	.add:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text);
	}
	.add:disabled {
		opacity: 0.4;
		cursor: default;
	}
</style>
