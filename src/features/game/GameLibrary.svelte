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
	// 목록 카드의 … 메뉴 (오른쪽 클릭도 같음)
	let cardMenu = $state<{ at: { x: number; y: number; align?: "start" | "end" }; items: MenuItem[] } | null>(null);
	function openCardMenu(e: MouseEvent, g: GameProject) {
		e.preventDefault();
		e.stopPropagation();
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		cardMenu = {
			at: e.type === "contextmenu" ? { x: e.clientX, y: e.clientY } : { x: r.right, y: r.bottom + 4, align: "end" },
			items: [
				{ label: "열기", icon: Play, onselect: () => nav.go({ page: "game", gameId: g.id }) },
				{ label: "편집", icon: Pencil, onselect: () => nav.go({ page: "game", gameId: g.id, edit: true }) },
				{ label: "내보내기 (다운로드 폴더)", icon: Download, onselect: async () => toasts.success(`내보냄 · ${await games.export(g.id)}`) },
				{
					label: "삭제",
					icon: Trash2,
					danger: true,
					separator: true,
					onselect: async () => {
						if (!(await confirmAction({ title: "게임 삭제", message: `'${g.title}'과 모든 플레이 기록을 휴지통으로 옮김. 설정 > 데이터 > 휴지통에서 되살릴 수 있음`, confirmLabel: "삭제", danger: true }))) return;
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
		if (!(await confirmAction({ title: "게임 삭제", message: `게임 ${ids.length}개와 그 저장·세션을 휴지통으로 옮김. 설정 > 데이터 > 휴지통에서 되살릴 수 있음`, confirmLabel: `${ids.length}개 삭제`, danger: true }))) return;
		for (const id of ids) await games.remove(id, false);
		selecting = false;
		toasts.success(`게임 ${ids.length}개를 휴지통으로 옮김`);
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
		if (g) toasts.success(`'${g.title}' 가져옴`);
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
		<div class="title-wrap">
			<h1>{$_("nav.game")}</h1>
			<span class="count">{games.list.length}{$_("common.unitItem")}</span>
		</div>
		<ListToolbar bind:query bind:filter bind:sort bind:selecting placeholder="제목·설명" filters={FILTERS} sorts={SORTS} canSelect={!!games.list.length}>
			<SplitButton label={$_("common.create")} onclick={startCreate} disabled={!games.templates.length} menuLabel={$_("common.import")} items={[{ label: "파일에서 가져오기", icon: Upload, onselect: importGame }]} />
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
					oncontextmenu={(e) => openCardMenu(e, g)}
				>
					{#snippet menu()}<IconButton label="{g.title} 메뉴" size="sm" onclick={(e) => openCardMenu(e, g)}><Ellipsis /></IconButton>{/snippet}
					{#snippet fallback()}<Gamepad2 />{/snippet}
				</ItemCard>
			{:else}
				<p class="none">검색 결과 없음</p>
			{/each}
		</div>
	{/if}
</div>

{#if cardMenu}<ContextMenu at={cardMenu.at} items={cardMenu.items} label="게임 메뉴" onclose={() => (cardMenu = null)} />{/if}

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
