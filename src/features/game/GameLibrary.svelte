<script lang="ts">
	import { Gamepad2, Plus, Search, Upload } from "@lucide/svelte";
	import { Button, Dialog, EmptyState, Field, Input, Select } from "$shared/ui";
	import { pickFiles, PACK_FILES } from "$shared/ipc/dialog";
	import { toasts } from "$shared/stores/toast.svelte";
	import { relative } from "$shared/time";
	import { nav } from "$app/nav.svelte";
	import { gameAssetUrl, games } from "./game.svelte";

	let query = $state("");
	const list = $derived(games.list.filter((g) => !query.trim() || g.title.toLowerCase().includes(query.trim().toLowerCase())));

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
		if (!path) return;
		const g = await games.import(path);
		toasts.success(`'${g.title}' 가져옴`);
	}
	const capLabel: Record<string, string> = { choices: "선택지", freeText: "자유 입력", inventory: "인벤토리", map: "지도", aiNarration: "AI 서술" };
</script>

<div class="page">
	<header class="head">
		<h1>게임</h1>
		<div class="tools">
			<Input bind:value={query} placeholder="제목 검색" aria-label="게임 검색">
				{#snippet leading()}<Search />{/snippet}
			</Input>
			<Button onclick={importGame}>{#snippet icon()}<Upload />{/snippet}가져오기</Button>
			<Button variant="primary" onclick={startCreate} disabled={!games.templates.length}>{#snippet icon()}<Plus />{/snippet}새 게임</Button>
		</div>
	</header>

	{#if !games.list.length}
		<EmptyState title="게임 없음" description="템플릿으로 새로 만들거나 게임 묶음을 가져옴">
			{#snippet icon()}<Gamepad2 />{/snippet}
			{#snippet action()}<Button variant="primary" onclick={startCreate}>새 게임</Button>{/snippet}
		</EmptyState>
	{:else}
		<div class="grid">
			{#each list as g (g.id)}
				{@const cover = gameAssetUrl(g.id, g.coverImage)}
				<button class="card" onclick={() => nav.go({ page: "game", gameId: g.id })}>
					<span class="cover" style:background-image={cover ? `url("${cover}")` : undefined}>
						{#if !cover}<Gamepad2 />{/if}
					</span>
					<span class="info">
						<b>{g.title}</b>
						<span class="muted desc">{g.description || "설명 없음"}</span>
						<span class="faint">{games.template(g.templateId)?.name ?? g.templateId} · v{g.version} · {relative(g.updatedAt)}</span>
					</span>
				</button>
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
	.tools {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		gap: var(--space-2);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: var(--space-4);
		max-width: 1100px;
		margin: 0 auto;
	}
	.card {
		display: flex;
		flex-direction: column;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		text-align: left;
		overflow: hidden;
		cursor: pointer;
		transition: border-color var(--duration-fast) var(--ease);
	}
	.card:hover {
		border-color: var(--accent);
	}
	.cover {
		display: grid;
		place-items: center;
		aspect-ratio: 16 / 9;
		background:
			radial-gradient(ellipse at 70% 0%, color-mix(in oklab, var(--accent) 40%, transparent), transparent 60%),
			var(--surface-active);
		background-size: cover;
		background-position: center;
		color: var(--text-faint);
	}
	.cover :global(svg) {
		width: 32px;
		height: 32px;
	}
	.info {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: var(--space-3) var(--space-4) var(--space-4);
	}
	.info span {
		font-size: var(--text-sm);
	}
	.desc {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
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
