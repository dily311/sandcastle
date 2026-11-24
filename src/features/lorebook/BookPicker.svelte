<script lang="ts">
	// 로어북 여러 권 고르기: 고른 책은 칩(× 로 빼기), '+'로 더 넣기
	import { BookOpen, Plus, X } from "@lucide/svelte";
	import { ContextMenu, type MenuItem } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
	import { lorebook } from "./lorebook.svelte";

	let { value, onchange, label, empty = "없음" }: { value: string[]; onchange: (ids: string[]) => void; label: string; empty?: string } = $props();

	// 모르는 책이 붙어 있으면(다른 곳에서 새로 생김, 예: 캐릭터 카드 가져오기) 목록을 다시 받음
	let reloaded = false;
	$effect(() => {
		if (!lorebook.loaded) return void lorebook.load();
		if (!reloaded && value.some((id) => !lorebook.book(id))) {
			reloaded = true;
			void lorebook.load();
		}
	});

	const picked = $derived(value.map((id) => lorebook.book(id)).filter((b) => !!b));
	let menu = $state<{ x: number; y: number } | null>(null);
	const items = $derived.by((): MenuItem[] => {
		const rest = lorebook.books.filter((b) => !value.includes(b.id));
		const list: MenuItem[] = rest.map((b) => ({ label: b.name, icon: BookOpen, onselect: () => onchange([...value, b.id]) }));
		list.push({ label: "로어북 화면에서 새로 만들기", icon: Plus, separator: list.length > 0, onselect: () => nav.go({ page: "lorebook" }) });
		return list;
	});
	function openMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.left, y: r.bottom + 4 };
	}
</script>

<div class="picker" role="group" aria-label={label}>
	{#each picked as b, i (i)}
		<span class="chip">
			<button class="name" title="'{b.name}' 로어북 열기" onclick={() => nav.go({ page: "lorebook", bookId: b.id })}>{b.name}</button>
			<button class="x" aria-label="{b.name} 빼기" onclick={() => onchange(value.filter((id) => id !== b.id))}><X /></button>
		</span>
	{:else}
		<span class="empty">{empty}</span>
	{/each}
	<button class="add" aria-label="{label}에 로어북 넣기" aria-haspopup="menu" onclick={openMenu}><Plus /></button>
</div>
<ContextMenu at={menu} label="로어북 넣기" {items} onclose={() => (menu = null)} />

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
	.chip button {
		border: none;
		background: transparent;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	.name {
		padding: 2px 2px 2px 8px;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.name:hover {
		text-decoration: underline;
	}
	.x {
		display: grid;
		place-items: center;
		width: 20px;
		height: 22px;
		padding: 0;
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
	.add:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
</style>
