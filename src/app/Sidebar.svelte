<script lang="ts">
	import { Gamepad2, Settings, Smartphone, UserRound, Users } from "@lucide/svelte";
	import { Avatar } from "$shared/ui";
	import { fileSrc } from "$shared/ipc/file";
	import { relative } from "$shared/time";
	import { library } from "$features/character/library.svelte";
	import { nav, type Page } from "./nav.svelte";

	// 화면이 늘어나면 여기에 추가
	const items: { page: Page; label: string; icon: typeof Users }[] = [
		{ page: "character", label: "캐릭터", icon: Users },
		{ page: "phone", label: "폰", icon: Smartphone },
		{ page: "persona", label: "페르소나", icon: UserRound },
		{ page: "game", label: "게임", icon: Gamepad2 },
	];

	$effect(() => {
		if (!library.loaded) void library.load();
	});

	// 최근 대화 (폰 대화 제외)
	const recent = $derived(
		library.conversations
			.filter((c) => c.mode !== "phone" && library.character(c.characterId))
			.sort((a, b) => b.updatedAt - a.updatedAt)
			.slice(0, 10),
	);
	const cur = $derived(nav.current);
	const openConv = $derived(cur.page === "character" ? cur.convId : undefined);
</script>

<aside class="sidebar" aria-label="앱 메뉴">
	<div class="brand"><span class="logo" aria-hidden="true">S</span><b>Sandcastle</b></div>

	<nav aria-label="화면">
		{#each items as it (it.page)}
			<button class="item" class:on={cur.page === it.page} aria-current={cur.page === it.page ? "page" : undefined} onclick={() => nav.go({ page: it.page } as never)}>
				<it.icon />{it.label}
			</button>
		{/each}
	</nav>

	{#if recent.length}
		<p class="group">최근 대화</p>
		<ul class="recent">
			{#each recent as c (c.id)}
				{@const ch = library.character(c.characterId)}
				<li>
					<button class:on={openConv === c.id} title="{ch?.name} · {c.title}" onclick={() => nav.go({ page: "character", charId: c.characterId ?? undefined, convId: c.id })}>
						<Avatar name={ch?.name ?? "?"} src={fileSrc(ch?.avatar)} color={ch?.color} size={22} />
						<span class="t">{c.title}</span>
						<span class="when">{relative(c.updatedAt)}</span>
					</button>
				</li>
			{/each}
		</ul>
	{/if}

	<div class="bottom">
		<button class="item" class:on={cur.page === "settings"} aria-current={cur.page === "settings" ? "page" : undefined} onclick={() => nav.go({ page: "settings" })}>
			<Settings />설정
		</button>
	</div>
</aside>

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		width: 220px;
		height: 100%;
		padding: var(--space-2);
		border-right: 1px solid var(--border);
		background: var(--bg);
		overflow: hidden;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-2) var(--space-3);
		font-size: var(--text-md);
	}
	.logo {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		border-radius: 6px;
		background: var(--accent);
		color: var(--text-on-accent);
		font-size: 12px;
		font-weight: 800;
	}
	nav {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	.item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		height: 32px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		cursor: pointer;
	}
	.item :global(svg) {
		width: 16px;
		height: 16px;
	}
	.item:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.item.on {
		background: var(--surface-active);
		color: var(--text);
		font-weight: 600;
	}
	.item.on :global(svg) {
		color: var(--accent);
	}
	.group {
		margin: var(--space-4) var(--space-2) var(--space-1);
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--text-faint);
	}
	.recent {
		flex: 1;
		min-height: 0;
		overflow: auto;
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.recent button {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		height: 30px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-sm);
		text-align: left;
		cursor: pointer;
	}
	.recent button:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.recent button.on {
		background: var(--surface-active);
		color: var(--text);
	}
	.t {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.when {
		flex: none;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.bottom {
		margin-top: auto;
		padding-top: var(--space-2);
		border-top: 1px solid var(--border);
	}
	.recent + .bottom {
		margin-top: 0;
	}
</style>
