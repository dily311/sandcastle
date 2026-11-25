<script lang="ts">
	// 목록 화면(캐릭터·게임·로어북·이미지) 공통 도구 줄: 찾기 · 거르기 · 정렬 · 여러 개 삭제 + 주 동작(children)
	import type { Snippet } from "svelte";
	import { _ } from "svelte-i18n";
	import { ArrowUpDown, Check, Filter, ListChecks, Search } from "@lucide/svelte";
	import IconButton from "./IconButton.svelte";
	import Input from "./Input.svelte";
	import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";

	export interface ListOption {
		value: string;
		label: string;
	}
	interface Props {
		query?: string;
		/** 찾기 칸 안내 */
		placeholder?: string;
		filters?: ListOption[];
		filter?: string;
		sorts?: ListOption[];
		sort?: string;
		/** 여러 개 선택 모드 (선택한 것을 지우거나 태그를 닮). canSelect가 없으면 버튼을 숨김 */
		selecting?: boolean;
		canSelect?: boolean;
		children?: Snippet;
	}
	let {
		query = $bindable(""),
		placeholder,
		filters = [],
		filter = $bindable("all"),
		sorts = [],
		sort = $bindable(""),
		selecting = $bindable(false),
		canSelect = false,
		children,
	}: Props = $props();
	const effectivePlaceholder = $derived(placeholder ?? $_("common.search"));

	let searchOpen = $state(false);
	let menu = $state<{ at: { x: number; y: number; align: "end" }; items: MenuItem[]; label: string } | null>(null);
	function open(e: MouseEvent, label: string, opts: ListOption[], cur: string, set: (v: string) => void) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { at: { x: r.right, y: r.bottom + 4, align: "end" }, label, items: opts.map((o) => ({ label: o.label, icon: o.value === cur ? Check : undefined, onselect: () => set(o.value) })) };
	}
	const activeFilterLabel = $derived(filter !== "all" ? filters.find((f) => f.value === filter)?.label : undefined);
	const sortActive = $derived(sorts.length > 1 && !!sort && sort !== sorts[0].value);
</script>

<div class="toolbar">
	{#if searchOpen || query}
		<div class="search">
			<Input bind:value={query} placeholder={effectivePlaceholder} aria-label={effectivePlaceholder} autofocus>{#snippet leading()}<Search />{/snippet}</Input>
		</div>
	{/if}
	{#if activeFilterLabel}
		<button type="button" class="filter-chip" title={$_("common.clearFilter")} onclick={() => (filter = "all")}>
			<span>{activeFilterLabel}</span>
			<span class="x" aria-hidden="true">×</span>
		</button>
	{/if}
	<IconButton size="sm" label={$_("common.search")} active={searchOpen || !!query} onclick={() => ((searchOpen = !searchOpen), !searchOpen && (query = ""))}><Search /></IconButton>
	{#if filters.length > 1}
		<IconButton size="sm" label="{$_('common.filter')} ({filters.find((f) => f.value === filter)?.label ?? ''})" active={filter !== "all"} onclick={(e) => open(e, $_("common.filter"), filters, filter, (v) => (filter = v))}><Filter /></IconButton>
	{/if}
	{#if sorts.length > 1}
		<IconButton size="sm" label="{$_('common.sort')} ({sorts.find((s) => s.value === sort)?.label ?? ''})" active={sortActive} onclick={(e) => open(e, $_("common.sort"), sorts, sort, (v) => (sort = v))}><ArrowUpDown /></IconButton>
	{/if}
	{#if canSelect}
		<IconButton size="sm" label={selecting ? $_("common.stopSelect") : $_("common.multiSelect")} active={selecting} onclick={() => (selecting = !selecting)}><ListChecks /></IconButton>
	{/if}
	{#if children}{@render children()}{/if}
</div>
{#if menu}<ContextMenu at={menu.at} label={menu.label} items={menu.items} onclose={() => (menu = null)} />{/if}

<style>
	.toolbar {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
		justify-content: flex-end;
	}
	.search {
		width: min(260px, 50vw);
	}
	.filter-chip {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		height: var(--control-sm);
		padding: 0 10px;
		border: 1px solid color-mix(in oklab, var(--accent) 45%, var(--border));
		border-radius: var(--radius-full);
		background: var(--accent-soft);
		color: var(--accent-text);
		font: inherit;
		font-size: var(--text-xs);
		font-weight: 600;
		cursor: pointer;
		transition: background var(--duration-fast) var(--ease);
	}
	.filter-chip:hover {
		background: var(--accent-soft-hover);
	}
	.filter-chip .x {
		font-size: 13px;
		line-height: 1;
		opacity: 0.75;
	}
</style>
