<script lang="ts">
	// 홈: 사용자가 꾸미는 첫 화면. 블록(이어하기·빠른 실행·즐겨찾기·게임·내 글)을 켜고 끄고 순서를 바꾼다.
	// AI를 부르지 않는다 (보여주기만). 설정은 앱 설정의 home에 저장
	import { lastAt } from "$features/character/library.svelte";
	import { _, locale } from "svelte-i18n";
	import { ArrowDown, ArrowUp, BookOpen, Check, Gamepad2, GripVertical, Images, MessageSquarePlus, Pencil, Smartphone, Star, UserRound, Users } from "@lucide/svelte";
	import { Sortable, moveItem } from "$shared/dnd/sortable.svelte";
	import type { HomeSetting_Serialize as HomeSetting } from "$ipc/bindings";
	import { Avatar, Button, EmptyState, Input, Switch, Textarea } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
	import { fileSrc } from "$shared/ipc/file";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { relative } from "$shared/time";
	import { library } from "$features/character/library.svelte";
	import { games } from "$features/game/game.svelte";
	import { fillNames } from "$features/character/format";

	const BLOCKS = $derived<{ id: string; label: string; desc: string; plugin?: "game" }[]>([
		{ id: "continue", label: $_("home.continue"), desc: $_("settings.chrome.homeBlockContinueDesc") },
		{ id: "quick", label: $_("home.quick"), desc: $_("settings.chrome.homeBlockQuickDesc") },
		{ id: "favorites", label: $_("home.favorites"), desc: $_("settings.chrome.homeBlockFavDesc") },
		{ id: "games", label: $_("home.games"), desc: $_("settings.chrome.homeBlockGamesDesc"), plugin: "game" },
	]);
	const FALLBACK: HomeSetting = { startOnHome: false, greeting: "", blocks: ["continue", "quick", "favorites", "games"], css: "" };

	$effect(() => {
		if (!library.loaded) void library.load();
		if (!appSetting.value) void appSetting.load();
	});
	$effect(() => {
		if (plugins.on("game") && !games.loaded) void games.load();
	});

	const home = $derived<HomeSetting>({ ...FALLBACK, ...((appSetting.value?.home as HomeSetting | undefined) ?? {}) });
	const me = $derived(library.persona(appSetting.value?.selectedPersonaId) ?? library.persona("default") ?? library.personas[0]);
	const greeting = $derived(fillNames(home.greeting.trim() || $_("home.greetingDefault"), { user: me?.name ?? "User" }));
	const today = $derived(new Date().toLocaleDateString(($locale ?? "ko").startsWith("en") ? "en-US" : "ko-KR", { month: "long", day: "numeric", weekday: "long" }));

	const recent = $derived(
		library.conversations
			.filter((c) => c.mode !== "phone" && library.character(c.characterId))
			.sort((a, b) => lastAt(b) - lastAt(a))
			.slice(0, 4),
	);
	const favorites = $derived(library.characters.filter((c) => c.favorite).slice(0, 12));
	const recentGames = $derived(games.list.slice(0, 4));
	const shown = $derived(home.blocks.filter((id) => BLOCKS.some((b) => b.id === id && (!b.plugin || plugins.on(b.plugin)))));

	async function newChat() {
		const ch = library.character(recent[0]?.characterId) ?? library.characters[0];
		if (!ch) return nav.go({ page: "character" });
		const c = await library.createConversation(ch.id);
		nav.go({ page: "character", charId: ch.id, convId: c.id });
	}

	// ---- 꾸미기 ----
	let editing = $state(false);
	let draft = $state<HomeSetting>(FALLBACK);
	function startEdit() {
		draft = structuredClone($state.snapshot(home));
		// 빠진 블록도 끄는 목록에 보이게 (켜진 것 먼저, 그 순서대로)
		editing = true;
	}
	// 전체 순서 (꺼진 블록은 켜진 것 뒤에). 끌어서 바꾸고, 켜진 것만 저장된다
	let order = $state<string[]>([]);
	$effect(() => {
		if (!editing) return;
		order = [...draft.blocks.filter((id) => BLOCKS.some((b) => b.id === id)), ...BLOCKS.map((b) => b.id).filter((id) => !draft.blocks.includes(id))];
	});
	function setOrder(next: string[]) {
		order = next;
		draft.blocks = next.filter((id) => draft.blocks.includes(id));
	}
	const sort = new Sortable((from, to) => setOrder(moveItem(order, from, to)));
	function toggle(id: string, on: boolean) {
		draft.blocks = on ? order.filter((x) => x === id || draft.blocks.includes(x)) : draft.blocks.filter((x) => x !== id);
	}
	function move(id: string, d: -1 | 1) {
		const i = order.indexOf(id);
		setOrder(moveItem(order, i, i + d));
	}
	// 홈에만 붙는 사용자 CSS (</style>로 빠져나가지 못하게)
	const customCss = $derived((editing ? draft.css : home.css).replace(/<\/style/gi, ""));
	async function saveEdit() {
		await appSetting.update({ home: $state.snapshot(draft) });
		editing = false;
	}
</script>

<svelte:head>
	{#if customCss.trim()}{@html `<style id="home-css">${customCss}</style>`}{/if}
</svelte:head>

<div class="home sc-home">
	<header class="hero">
		<div>
			<p class="date">{today}</p>
			<h1>{greeting}</h1>
		</div>
		{#if !editing}
			<Button size="sm" variant="secondary" onclick={startEdit}>{#snippet icon()}<Pencil />{/snippet}{$_("home.customize")}</Button>
		{/if}
	</header>

	{#if editing}
		<section class="card edit">
			<h2>{$_("home.customizeTitle")}</h2>
			<label class="field"><span>{$_("settings.chrome.homeGreetingLabel")} <small>{$_("settings.chrome.homeGreetingHint")}</small></span><Input bind:value={draft.greeting} placeholder={$_("home.greetingDefault")} /></label>
			<div class="field">
				<span>{$_("settings.chrome.homeBlocksLabel")}</span>
				<ul class="blocks">
					{#each order as id, i (id)}
						{@const b = BLOCKS.find((x) => x.id === id)!}
						{@const on = draft.blocks.includes(id)}
						<li {...sort.item(i)} class:dragging={sort.from === i} class:over={sort.over === i && sort.from !== i}>
							<span class="grip" aria-hidden="true" {...sort.handle(i)}><GripVertical /></span>
							<Switch checked={on} label={b.label} hideLabel onchange={(v) => toggle(id, v)} />
							<span class="bl"><b>{b.label}</b><small>{b.desc}{b.plugin && !plugins.on(b.plugin) ? $_("settings.chrome.homePluginOff") : ""}</small></span>
						</li>
					{/each}
				</ul>
			</div>
			<label class="field">
				<span>{$_("settings.chrome.homeCssLabel")} <small>{$_("settings.chrome.homeCssHint")}</small></span>
				<Textarea bind:value={draft.css} autosize rows={4} maxRows={16} mono placeholder={".home { background: linear-gradient(#0000, var(--accent-soft)); }\n.home .card { border-radius: 24px; }"} />
			</label>
			<label class="row"><Switch checked={draft.startOnHome} label={$_("settings.chrome.homeStartOnHome")} hideLabel onchange={(v) => (draft.startOnHome = v)} /><span>{$_("settings.chrome.homeStartOnHome")}</span></label>
			<div class="btns">
				<Button variant="ghost" onclick={() => (editing = false)}>{$_("common.cancel")}</Button>
				<Button variant="primary" onclick={saveEdit}>{#snippet icon()}<Check />{/snippet}{$_("common.save")}</Button>
			</div>
		</section>
	{/if}

	{#each shown as id (id)}
		{#if id === "continue"}
			<section>
				<h2>{$_("home.continue")}</h2>
				{#if recent.length}
					<div class="grid">
						{#each recent as c (c.id)}
							{@const ch = library.character(c.characterId)}
							<button class="card conv" onclick={() => nav.go({ page: "character", charId: c.characterId ?? undefined, convId: c.id })}>
								<Avatar name={ch?.name ?? "?"} src={fileSrc(ch?.avatar)} fit={ch?.avatarFit} color={ch?.color} size={36} />
								<span class="txt"><b>{c.title}</b><small>{ch?.name} · {relative(lastAt(c))}</small></span>
							</button>
						{/each}
					</div>
				{:else}
					<p class="muted">{$_("home.noChats")}</p>
				{/if}
			</section>
		{:else if id === "quick"}
			<section>
				<h2>{$_("home.quick")}</h2>
				<div class="quick">
					<button class="card q" onclick={newChat}><MessageSquarePlus /><span>{$_("home.newChat")}</span></button>
					<button class="card q" onclick={() => nav.go({ page: "character" })}><Users /><span>{$_("nav.character")}</span></button>
					{#if plugins.on("phone")}<button class="card q" onclick={() => nav.go({ page: "phone" })}><Smartphone /><span>{$_("nav.phone")}</span></button>{/if}
					{#if plugins.on("game")}<button class="card q" onclick={() => nav.go({ page: "game" })}><Gamepad2 /><span>{$_("nav.game")}</span></button>{/if}
					<button class="card q" onclick={() => nav.go({ page: "lorebook" })}><BookOpen /><span>{$_("nav.lorebook")}</span></button>
					<button class="card q" onclick={() => nav.go({ page: "images" })}><Images /><span>{$_("nav.images")}</span></button>
					<button class="card q" onclick={() => nav.go({ page: "persona" })}><UserRound /><span>{$_("nav.persona")}</span></button>
				</div>
			</section>
		{:else if id === "favorites"}
			<section>
				<h2>{$_("home.favorites")}</h2>
				{#if favorites.length}
					<div class="favs">
						{#each favorites as ch (ch.id)}
							<button class="card fav" onclick={() => nav.go({ page: "character", charId: ch.id })} title={ch.name}>
								<Avatar name={ch.name} src={fileSrc(ch.avatar)} fit={ch.avatarFit} color={ch.color} size={44} />
								<span>{ch.name}</span>
							</button>
						{/each}
					</div>
				{:else}
					<p class="muted"><Star size={14} /> {$_("home.favHint")}</p>
				{/if}
			</section>
		{:else if id === "games"}
			<section>
				<h2>{$_("home.games")}</h2>
				{#if recentGames.length}
					<div class="grid">
						{#each recentGames as g (g.id)}
							<button class="card conv" onclick={() => nav.go({ page: "game", gameId: g.id })}>
								<Gamepad2 />
								<span class="txt"><b>{g.title}</b><small>{relative(g.updatedAt)}</small></span>
							</button>
						{/each}
					</div>
				{:else}
					<p class="muted">{$_("home.noGames")}</p>
				{/if}
			</section>
		{/if}
	{/each}
</div>

<style>
	.home {
		max-width: var(--page-wide);
		margin: 0 auto;
		padding: var(--space-6) var(--space-8) var(--space-10);
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	.hero {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: var(--space-3);
	}
	.date {
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	h1 {
		font-size: var(--text-2xl, 1.75rem);
		margin-top: var(--space-1);
	}
	h2 {
		font-size: var(--text-sm);
		color: var(--text-muted);
		font-weight: 600;
		margin-bottom: var(--space-2);
	}
	.card {
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
	}
	button.card {
		font: inherit;
		color: var(--text);
		cursor: pointer;
		text-align: left;
		transition:
			background var(--duration-fast) var(--ease),
			border-color var(--duration-fast) var(--ease),
			box-shadow var(--duration-fast) var(--ease),
			transform var(--duration-fast) var(--ease);
	}
	button.card:hover {
		background: var(--surface-hover);
		border-color: var(--border-strong);
		box-shadow: var(--shadow-sm);
		transform: translateY(-1px);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(210px, 45%), 1fr));
		gap: var(--space-2);
	}
	.conv {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3);
		min-width: 0;
	}
	.conv :global(svg) {
		width: 22px;
		height: 22px;
		color: var(--text-muted);
		flex-shrink: 0;
	}
	.txt {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.txt b,
	.txt small {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.txt small {
		color: var(--text-muted);
	}
	.quick {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.q {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
	}
	.q :global(svg) {
		width: 18px;
		height: 18px;
		color: var(--accent-text);
	}
	.favs {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}
	.fav {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-2);
		width: 88px;
		padding: var(--space-3) var(--space-2);
		text-align: center;
		font-size: var(--text-xs);
		font-weight: 500;
	}
	.fav span {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.muted {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.edit {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-5);
	}
	.edit h2 {
		font-size: var(--text-md, 1rem);
		color: var(--text);
		margin: 0;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.field small {
		color: var(--text-faint);
		margin-left: 4px;
	}
	.blocks {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.blocks li {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-2);
		border-radius: var(--radius-md);
		cursor: grab;
	}
	.blocks li.dragging {
		opacity: 0.4;
	}
	.blocks li.over {
		box-shadow: inset 0 2px 0 var(--accent);
	}
	.grip {
		display: grid;
		color: var(--text-faint);
	}
	.grip :global(svg) {
		width: 16px;
		height: 16px;
	}
	.bl {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.bl small {
		color: var(--text-faint);
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.btns {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
	}
	@media (max-width: 600px) {
		.home {
			padding: var(--space-4) var(--space-4) var(--space-6);
		}
	}
</style>
