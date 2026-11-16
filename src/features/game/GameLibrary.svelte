<script lang="ts">
	import { drops, extOf, storeFile } from "$app/drop.svelte";
	import { Gamepad2, Plus, Search, Upload } from "@lucide/svelte";
	import { Button, Dialog, EmptyState, Field, Input, ItemCard, ListToolbar, Select, SelectBar, SplitButton } from "$shared/ui";
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
		{ value: "all", label: `전부 ${games.list.length}` },
		...[...new Set(games.list.map((g) => g.templateId))].map((t) => ({ value: `tpl:${t}`, label: games.template(t)?.name ?? t })),
	]);
	const SORTS = [
		{ value: "recent", label: "최근 바뀐 순" },
		{ value: "name", label: "제목 순" },
		{ value: "new", label: "최근 만든 순" },
	];
	const list = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const by = sort === "name" ? (a: GameProject, b: GameProject) => a.title.localeCompare(b.title, "ko") : sort === "new" ? (a: GameProject, b: GameProject) => b.createdAt - a.createdAt : (a: GameProject, b: GameProject) => b.updatedAt - a.updatedAt;
		return games.list.filter((g) => (filter === "all" || g.templateId === filter.slice(4)) && (!q || g.title.toLowerCase().includes(q) || (g.description ?? "").toLowerCase().includes(q))).sort(by);
	});
	function pickGame(id: string) {
		const next = new Set(picked);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		picked = next;
	}
	async function removePicked() {
		const ids = [...picked];
		if (!ids.length) return;
		if (!(await confirmAction({ title: "게임 삭제", message: `게임 ${ids.length}개와 그 저장·세션을 지움. 되돌릴 수 없음`, confirmLabel: `${ids.length}개 삭제`, danger: true }))) return;
		for (const id of ids) await games.remove(id);
		selecting = false;
		toasts.success(`게임 ${ids.length}개 지움`);
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
		const [path] = await pickFiles([{ name: "게임 묶음", extensions: ["zip", "tgame"] }, PACK_FILES], false);
		if (path) await importPath(path);
	}
	async function importPath(path: string) {
		const g = await games.import(path);
		toasts.success(`'${g.title}' 가져옴`);
	}
	$effect(() =>
		drops.register({
			label: "게임 가져오기",
			hint: "게임 묶음 (.zip, .tgame)",
			accept: (f) => ["zip", "tgame"].includes(extOf(f)),
			ondrop: async (files) => {
				for (const f of files) await importPath(await storeFile(f)).catch(() => {});
			},
		}),
	);
	const capLabel: Record<string, string> = { choices: "선택지", freeText: "자유 입력", inventory: "인벤토리", map: "지도", aiNarration: "AI 서술" };
</script>

<div class="page">
	<header class="head">
		<h1>게임</h1>
		<ListToolbar bind:query bind:filter bind:sort bind:selecting placeholder="제목·설명" filters={FILTERS} sorts={SORTS} canSelect={!!games.list.length}>
			<SplitButton label="새로 만들기" onclick={startCreate} disabled={!games.templates.length} menuLabel="게임 가져오기" items={[{ label: "파일에서 가져오기", icon: Upload, onselect: importGame }]} />
		</ListToolbar>
	</header>
	{#if selecting}<SelectBar count={picked.size} onall={() => (picked = new Set(list.map((g) => g.id)))} ondelete={removePicked} />{/if}

	{#if !games.list.length}
		<EmptyState title="게임 없음" description="템플릿으로 새로 만들거나 게임 묶음을 가져옴">
			{#snippet icon()}<Gamepad2 />{/snippet}
		</EmptyState>
	{:else}
		<div class="grid">
			{#each list as g (g.id)}
				<ItemCard
					title={g.title}
					desc={g.description || "설명 없음"}
					meta="{games.template(g.templateId)?.name ?? g.templateId} · v{g.version} · {relative(g.updatedAt)}"
					image={gameAssetUrl(g.id, g.coverImage)}
					{selecting}
					selected={picked.has(g.id)}
					onclick={() => (selecting ? pickGame(g.id) : nav.go({ page: "game", gameId: g.id }))}
				>
					{#snippet fallback()}<Gamepad2 />{/snippet}
				</ItemCard>
			{:else}
				<p class="none">검색 결과 없음</p>
			{/each}
		</div>
	{/if}
</div>

<Dialog bind:open={createOpen} title="새 게임" description="템플릿이 게임 규칙과 시작 데이터를 정함" size="sm">
	<div class="form">
		<Field label="제목" required><Input bind:value={title} placeholder="게임 제목" /></Field>
		<Field label="템플릿">
			<Select bind:value={templateId} options={games.templates.map((t) => ({ value: t.id, label: t.name }))} />
		</Field>
		{#if games.template(templateId)}
			{@const caps = games.template(templateId)!.capabilities as unknown as Record<string, boolean>}
			<p class="faint caps">{Object.entries(capLabel).filter(([k]) => caps[k]).map(([, v]) => v).join(" · ") || "기능 정보 없음"}</p>
		{/if}
	</div>
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (createOpen = false)}>취소</Button>
		<Button variant="primary" loading={creating} disabled={!title.trim() || !templateId} onclick={create}>만들기</Button>
	{/snippet}
</Dialog>

<style>
	.page {
		height: 100%;
		overflow: auto;
		padding: var(--space-6) var(--space-8);
	}
	.head {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		max-width: 1100px;
		margin: 0 auto var(--space-5);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: var(--space-4);
		max-width: 1100px;
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
</style>
