<script lang="ts">
	import { drops, extOf, storeFile } from "$app/drop.svelte";
	import { _ } from "svelte-i18n";
	import { Download, Ellipsis, Gamepad2, Pencil, Play, Plus, Search, Trash2, Upload } from "@lucide/svelte";
	import { Button, ContextMenu, Dialog, EmptyState, Field, IconButton, Input, ItemCard, ListToolbar, Select, SelectBar, SplitButton, type MenuItem } from "$shared/ui";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { pickFiles, PACK_FILES } from "$shared/ipc/dialog";
	import { toasts } from "$shared/stores/toast.svelte";
	import { relative } from "$shared/time";
	import { nav } from "$app/nav.svelte";
	import { gameAssetUrl, games } from "./game.svelte";
	import type { GameProject_Serialize as GameProject } from "$ipc/bindings";

	// ---- 찾기 · 거르기 · 정렬 · 여러 개 삭제 (다른 목록 화면과 같은 도구) ----
	let query = $state("");
	let filter = $state("all");
	let sort = $state("recent");
	let selecting = $state(false);
	let picked = $state<Set<string>>(new Set());
	$effect(() => {
		if (!selecting) picked = new Set();
	});
	const FILTERS = $derived([
		{ value: "all", label: $_("gameLib.filterAll", { values: { count: games.list.length } }) },
		...[...new Set(games.list.map((g) => g.templateId))].map((t) => ({ value: `tpl:${t}`, label: games.template(t)?.name ?? t })),
	]);
	const SORTS = $derived([
		{ value: "recent", label: $_("gameLib.sortRecent") },
		{ value: "name", label: $_("gameLib.sortName") },
		{ value: "new", label: $_("gameLib.sortNew") },
	]);
	const list = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const by = sort === "name" ? (a: GameProject, b: GameProject) => a.title.localeCompare(b.title, "ko") : sort === "new" ? (a: GameProject, b: GameProject) => b.createdAt - a.createdAt : (a: GameProject, b: GameProject) => b.updatedAt - a.updatedAt;
		return games.list.filter((g) => (filter === "all" || g.templateId === filter.slice(4)) && (!q || g.title.toLowerCase().includes(q) || (g.description ?? "").toLowerCase().includes(q))).sort(by);
	});
	// 목록 카드의 … 메뉴 (오른쪽 클릭도 같음)
	let cardMenu = $state<{ at: { x: number; y: number; align?: "start" | "end" }; items: MenuItem[] } | null>(null);
	function openCardMenu(e: MouseEvent, g: GameProject) {
		e.preventDefault();
		e.stopPropagation();
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		cardMenu = {
			at: e.type === "contextmenu" ? { x: e.clientX, y: e.clientY } : { x: r.right, y: r.bottom + 4, align: "end" },
			items: [
				{ label: $_("gameLib.openMenu"), icon: Play, onselect: () => nav.go({ page: "game", gameId: g.id }) },
				{ label: $_("gameLib.editMenu"), icon: Pencil, onselect: () => nav.go({ page: "game", gameId: g.id, edit: true }) },
				{ label: $_("gameLib.exportMenu"), icon: Download, onselect: async () => toasts.success($_("gameLib.exportedToast", { values: { path: await games.export(g.id) } })) },
				{
					label: $_("common.delete"),
					icon: Trash2,
					danger: true,
					separator: true,
					onselect: async () => {
						if (!(await confirmAction({ title: $_("gameLib.deleteTitle"), message: $_("gameLib.deleteOneMsg", { values: { title: g.title } }), confirmLabel: $_("common.delete"), danger: true }))) return;
						await games.remove(g.id);
					},
				},
			],
		};
	}
	function pickGame(id: string) {
		const next = new Set(picked);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		picked = next;
	}
	async function removePicked() {
		const ids = [...picked];
		if (!ids.length) return;
		if (!(await confirmAction({ title: $_("gameLib.deleteTitle"), message: $_("gameLib.deleteManyMsg", { values: { count: ids.length } }), confirmLabel: $_("gameLib.deleteCountBtn", { values: { count: ids.length } }), danger: true }))) return;
		for (const id of ids) await games.remove(id, false);
		selecting = false;
		toasts.success($_("gameLib.deletedManyToast", { values: { count: ids.length } }));
	}

	// ---- 새 게임 ----
	let createOpen = $state(false);
	let title = $state("");
	let templateId = $state("");
	let creating = $state(false);
	function startCreate() {
		title = "";
		templateId = games.templates[0]?.id ?? "";
		createOpen = true;
	}
	async function create() {
		if (!title.trim() || !templateId) return;
		creating = true;
		try {
			const g = await games.create(title.trim(), templateId);
			createOpen = false;
			nav.go({ page: "game", gameId: g.id });
		} finally {
			creating = false;
		}
	}
	async function importGame() {
		const [path] = await pickFiles([{ name: $_("gameLib.packFileLabel"), extensions: ["zip", "tgame"] }, PACK_FILES], false);
		if (path) await importPath(path);
	}
	async function importPath(path: string) {
		const g = await games.import(path);
		if (g) toasts.success($_("gameLib.importedToast", { values: { title: g.title } }));
	}
	$effect(() =>
		drops.register({
			label: $_("gameLib.dropLabel"),
			hint: $_("gameLib.dropHint"),
			accept: (f) => ["zip", "tgame"].includes(extOf(f)),
			ondrop: async (files) => {
				for (const f of files) await importPath(await storeFile(f)).catch(() => {});
			},
		}),
	);
	const capLabel = $derived<Record<string, string>>({
		choices: $_("gameLib.capChoices"),
		freeText: $_("gameLib.capFreeText"),
		inventory: $_("gameLib.capInventory"),
		map: $_("gameLib.capMap"),
		aiNarration: $_("gameLib.capAiNarration"),
	});
</script>

<div class="page">
	<header class="head">
		<div class="title-wrap">
			<h1>{$_("nav.game")}</h1>
			<span class="count">{games.list.length}{$_("common.unitItem")}</span>
		</div>
		<ListToolbar bind:query bind:filter bind:sort bind:selecting placeholder={$_("gameLib.searchPlaceholder")} filters={FILTERS} sorts={SORTS} canSelect={!!games.list.length}>
			<SplitButton label={$_("common.create")} onclick={startCreate} disabled={!games.templates.length} menuLabel={$_("common.import")} items={[{ label: $_("gameLib.importFile"), icon: Upload, onselect: importGame }]} />
		</ListToolbar>
	</header>
	{#if selecting}<SelectBar count={picked.size} onall={() => (picked = new Set(list.map((g) => g.id)))} ondelete={removePicked} />{/if}

	{#if !games.list.length}
		<EmptyState title={$_("gameLib.emptyTitle")} description={$_("gameLib.emptyDesc")}>
			{#snippet icon()}<Gamepad2 />{/snippet}
		</EmptyState>
	{:else}
		<div class="grid">
			{#each list as g (g.id)}
				<ItemCard
					title={g.title}
					desc={g.description || $_("gameLib.noDesc")}
					meta="{games.template(g.templateId)?.name ?? g.templateId} · v{g.version} · {relative(g.updatedAt)}"
					image={gameAssetUrl(g.id, g.coverImage)}
					{selecting}
					selected={picked.has(g.id)}
					onclick={() => (selecting ? pickGame(g.id) : nav.go({ page: "game", gameId: g.id }))}
					oncontextmenu={(e) => openCardMenu(e, g)}
				>
					{#snippet menu()}<IconButton label={$_("gameLib.itemMenuAria", { values: { title: g.title } })} size="sm" onclick={(e) => openCardMenu(e, g)}><Ellipsis /></IconButton>{/snippet}
					{#snippet fallback()}<Gamepad2 />{/snippet}
				</ItemCard>
			{:else}
				<p class="none">{$_("gameLib.noResults")}</p>
			{/each}
		</div>
	{/if}
</div>

{#if cardMenu}<ContextMenu at={cardMenu.at} items={cardMenu.items} label={$_("gameLib.menuAria")} onclose={() => (cardMenu = null)} />{/if}

<Dialog bind:open={createOpen} title={$_("gameLib.newTitle")} description={$_("gameLib.newDesc")} size="sm">
	<div class="form">
		<Field label={$_("gameLib.titleField")} required><Input bind:value={title} placeholder={$_("gameLib.titlePlaceholder")} /></Field>
		<Field label={$_("gameLib.templateField")}>
			<Select bind:value={templateId} options={games.templates.map((t) => ({ value: t.id, label: t.name }))} />
		</Field>
		{#if games.template(templateId)}
			{@const caps = games.template(templateId)!.capabilities as unknown as Record<string, boolean>}
			<p class="faint caps">{Object.entries(capLabel).filter(([k]) => caps[k]).map(([, v]) => v).join(" · ") || $_("gameLib.noCapInfo")}</p>
		{/if}
	</div>
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (createOpen = false)}>{$_("common.cancel")}</Button>
		<Button variant="primary" loading={creating} disabled={!title.trim() || !templateId} onclick={create}>{$_("gameLib.createBtn")}</Button>
	{/snippet}
</Dialog>

<style>
	.page {
		--selectbar-max: var(--page-wide);
		height: 100%;
		overflow: auto;
		padding: var(--space-6) var(--space-8);
	}
	.head {
		display: flex;
		flex-direction: row;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		/* 좁은 화면: 도구 줄이 제목 아래로 내려감 (제목이 "캐릭/터"처럼 잘리지 않게) */
		flex-wrap: wrap;
		max-width: var(--page-wide);
		margin: 0 auto var(--space-5);
	}
	.title-wrap {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
	}
	.head h1 {
		white-space: nowrap;
	}
	.count {
		color: var(--text-faint);
		font-size: var(--text-sm);
		font-variant-numeric: tabular-nums;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(220px, 45%), 1fr));
		gap: var(--space-3);
		max-width: var(--page-wide);
		margin: 0 auto;
	}
	.none {
		color: var(--text-faint);
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.caps {
		font-size: var(--text-xs);
	}
	/* 휴대폰 폭: 좌우 여백을 홈과 같게 줄여 도구 줄(찾기·거르기·정렬·선택 + 새로 만들기)이 한 줄에 들어가게 */
	@media (max-width: 600px) {
		.page {
			padding: var(--space-4);
		}
	}
</style>
