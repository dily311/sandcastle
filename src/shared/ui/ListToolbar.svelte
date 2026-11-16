<script lang="ts">
	// 목록 화면(캐릭터·게임·로어북·이미지) 공통 도구 줄: 찾기 · 거르기 · 정렬 · 여러 개 삭제 + 주 동작(children)
	import type { Snippet } from "svelte";
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
		placeholder = "찾기",
		filters = [],
		filter = $bindable("all"),
		sorts = [],
		sort = $bindable(""),
		selecting = $bindable(false),
		canSelect = false,
		children,
	}: Props = $props();

	let searchOpen = $state(false);
	let menu = $state<{ at: { x: number; y: number; align: "end" }; items: MenuItem[]; label: string } | null>(null);
	function open(e: MouseEvent, label: string, opts: ListOption[], cur: string, set: (v: string) => void) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { at: { x: r.right, y: r.bottom + 4, align: "end" }, label, items: opts.map((o) => ({ label: o.label, icon: o.value === cur ? Check : undefined, onselect: () => set(o.value) })) };
	}
</script>

<div class="toolbar">
	{#if searchOpen || query}
		<div class="search">
			<Input bind:value={query} {placeholder} aria-label={placeholder} autofocus>{#snippet leading()}<Search />{/snippet}</Input>
		</div>
	{/if}
	<IconButton size="sm" label="찾기" active={searchOpen || !!query} onclick={() => ((searchOpen = !searchOpen), !searchOpen && (query = ""))}><Search /></IconButton>
	{#if filters.length > 1}
		<IconButton size="sm" label="거르기 (지금: {filters.find((f) => f.value === filter)?.label ?? ""})" active={filter !== "all"} onclick={(e) => open(e, "거르기", filters, filter, (v) => (filter = v))}><Filter /></IconButton>
	{/if}
	{#if sorts.length > 1}
		<IconButton size="sm" label="정렬 (지금: {sorts.find((s) => s.value === sort)?.label ?? ""})" onclick={(e) => open(e, "정렬", sorts, sort, (v) => (sort = v))}><ArrowUpDown /></IconButton>
	{/if}
	{#if canSelect}
		<IconButton size="sm" label={selecting ? "선택 끝내기" : "여러 개 선택"} active={selecting} onclick={() => (selecting = !selecting)}><ListChecks /></IconButton>
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
</style>
