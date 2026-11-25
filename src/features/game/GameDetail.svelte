<script lang="ts">
	import { _ } from "svelte-i18n";
	import { onMount } from "svelte";
	import { AlertTriangle, ImagePlus, X, ChevronRight, Download, Ellipsis, Gamepad2, MessageSquare, Pencil, Play, Plus, Trash2, Users } from "@lucide/svelte";
	import { Avatar, Badge, Button, ContextMenu, EmptyState, IconButton, ListToolbar, Segmented, Spinner, type MenuItem } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { GAME_FILES, IMAGE_FILES, pickFiles, saveIntoPicked, saveTo } from "$shared/ipc/dialog";
	import { layout } from "$app/layout.svelte";
	import { dropPaths } from "$app/drop.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { absolute, relative } from "$shared/time";
	import { nav } from "$app/nav.svelte";
	import { library } from "$features/character/library.svelte";
	import { newGameSession, startGameSession } from "$features/game/runtime/session";
	import { gameAssetUrl, games, type Issue, type Session } from "./game.svelte";
	import { dataDiagnostics, reportGameError } from "./errors";

	let { gameId }: { gameId: string } = $props();
	let loading = $state(true);
	let failed = $state(false);

	onMount(async () => {
		try {
			await games.open(gameId);
		} catch {
			failed = true;
		} finally {
			loading = false;
		}
	});

	const game = $derived(games.opened?.project.id === gameId ? games.opened : null);
	const view = $derived((game?.view ?? {}) as {
		scenes?: { id: string; name: string | null; sceneKey: string }[];
		characters?: { id: string; name: string; avatar: string | null; description: string | null }[];
	});
	const errors = $derived((game?.diagnostics ?? []).filter((d: Issue) => d.severity === "error"));
	const warnings = $derived((game?.diagnostics ?? []).filter((d: Issue) => d.severity !== "error"));
	const template = $derived(game ? games.template(game.project.templateId) : undefined);
	const cover = $derived(game ? gameAssetUrl(gameId, game.project.coverImage) : null);

	// 커버 바로 바꾸기 (게임 폴더 cover/에 복사)
	async function changeCover(remove = false) {
		let path: string | null = null;
		if (!remove) {
			[path] = await pickFiles([IMAGE_FILES], false);
			if (!path) return;
		}
		await setCover(path);
	}
	async function setCover(path: string | null) {
		const saved = await call(commands.setGameCover(gameId, path), { source: "game" });
		if (games.opened?.project.id === gameId) games.opened.project.coverImage = saved.coverImage;
		const g = games.game(gameId);
		if (g) g.coverImage = saved.coverImage;
	}
	// 끌어다 놓은 그림은 이 게임의 커버로 (커버 버튼과 같은 처리)
	$effect(() => dropPaths({ label: $_("gameUi.dropCoverLabel"), hint: $_("gameUi.dropCoverHint"), multiple: false, onpaths: ([path]) => setCover(path) }));

	let tab = $state<"sessions" | "characters" | "check">("sessions");
	let starting = $state(false);
	let sessionQuery = $state("");
	let sessionSort = $state("recent");
	let charQuery = $state("");
	let charSort = $state("name");
	const SESSION_SORTS = $derived([
		{ value: "recent", label: $_("gameUi.sortRecent") },
		{ value: "turns", label: $_("gameUi.sortTurns") },
		{ value: "oldest", label: $_("gameUi.sortOldest") },
	]);
	const CHAR_SORTS = $derived([
		{ value: "name", label: $_("gameUi.sortName") },
		{ value: "order", label: $_("gameUi.sortOrder") },
	]);
	const filteredSessions = $derived.by(() => {
		const q = sessionQuery.trim().toLowerCase();
		const list = games.sessions.filter((s) => !q || sceneName(s.currentSceneId).toLowerCase().includes(q));
		if (sessionSort === "turns") return [...list].sort((a, b) => b.headSeq - a.headSeq);
		if (sessionSort === "oldest") return [...list].sort((a, b) => a.updatedAt - b.updatedAt);
		return [...list].sort((a, b) => b.updatedAt - a.updatedAt);
	});
	const filteredCharacters = $derived.by(() => {
		const q = charQuery.trim().toLowerCase();
		const list = (view.characters ?? []).filter((c) => !q || c.name.toLowerCase().includes(q) || (c.description ?? "").toLowerCase().includes(q));
		if (charSort === "name") return [...list].sort((a, b) => a.name.localeCompare(b.name, "ko"));
		return list;
	});

	async function startNew() {
		if (errors.length) {
			tab = "check";
			return toasts.error($_("gameUi.dataErrorToast"));
		}
		starting = true;
		try {
			const sid = await newGameSession(gameId);
			nav.go({ page: "game", gameId, sessionId: sid });
		} catch (e) {
			if (dataDiagnostics(e).length) {
				// 열었을 때 이후로 데이터가 바뀌었을 수 있다: 진단을 다시 받아 진단 탭에서 보여준다
				tab = "check";
				await games.open(gameId).catch(() => {});
			}
			reportGameError(e, gameId);
		} finally {
			starting = false;
		}
	}
	async function resume(s: Session) {
		starting = true;
		try {
			await startGameSession(gameId, s.id);
			nav.go({ page: "game", gameId, sessionId: s.id });
		} catch (e) {
			toasts.error($_("gameUi.sessionLoadErr"), String((e as Error)?.message ?? e));
		} finally {
			starting = false;
		}
	}
	async function removeSession(s: Session) {
		if (await confirmAction({ title: $_("gameUi.deleteSessionTitle"), message: $_("gameUi.deleteSessionMsg", { values: { when: relative(s.updatedAt) } }), confirmLabel: $_("common.delete"), danger: true })) await games.removeSession(s.id);
	}
	const sceneName = (id: string | null) => view.scenes?.find((x) => x.id === id)?.name ?? view.scenes?.find((x) => x.id === id)?.sceneKey ?? $_("gameUi.startScene");
	const clock = (sec = 0) => {
		const m = Math.floor(sec / 60);
		return `${Math.floor(m / 60) % 24}:${String(m % 60).padStart(2, "0")}`;
	};

	/** 게임 → "세계" 캐릭터 + 분류된 로어북 (+ 플레이 기록을 "여러 캐릭터" 대화로) (5-F-3). 원본 게임은 그대로 */
	let copying = $state(false);
	async function copyToChat(s: Session | null) {
		if (copying) return;
		copying = true;
		try {
			const r = await call(commands.copyGameToCharacter(gameId, s?.id ?? null), { source: "game" });
			await library.load();
			const convId = r.conversation?.id;
			toasts.show({
				tone: "success",
				message: convId ? $_("gameUi.copiedChatToast", { values: { name: r.character.name } }) : $_("gameUi.copiedCharBookToast", { values: { name: r.character.name } }),
				action: { label: $_("gameUi.openBtn"), run: () => nav.go({ page: "character", charId: r.character.id, convId }) },
			});
		} finally {
			copying = false;
		}
	}

	/** 플레이 기록 → SillyTavern 채팅 (5-E-20-3). 연출 태그는 걷고 선택지는 줄로 */
	async function exportSession(s: Session) {
		const title = `${game?.project.title ?? "game"} - ${sceneName(s.currentSceneId)}`;
		const n = await saveTo(`${title}.jsonl`, [{ name: $_("gameUi.stChatFile"), extensions: ["jsonl"] }], (path) => call(commands.exportGameSession(s.id, path), { source: "game" }));
		if (n != null) toasts.success($_("gameUi.exportedMsgsToast", { values: { count: n } }));
	}

	async function copyToCharacter(id: string, name: string) {
		const c = await call(commands.copyGameCharacterToCharacter(id), { source: "game" });
		library.characters = [...library.characters, c];
		toasts.show({ tone: "success", message: $_("gameUi.copiedCharToast", { values: { name } }), action: { label: $_("gameUi.openBtn"), run: () => nav.go({ page: "character", charId: c.id }) } });
	}

	let more = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	const moreItems = $derived<MenuItem[]>([
		{ label: layout.desktop ? $_("gameUi.exportDownload") : $_("gameUi.exportBtn"), icon: Download, onselect: exportGame },
		{ label: $_("gameUi.copyToCharBook"), icon: Users, onselect: () => void copyToChat(null) },
		{ label: $_("common.delete"), icon: Trash2, danger: true, separator: true, onselect: removeGame },
	]);
	async function exportGame() {
		// Android는 공용 다운로드 폴더에 직접 쓸 수 없어 저장 위치를 고른다
		const path = layout.desktop ? await games.export(gameId) : await saveIntoPicked(`${game?.project.title ?? "game"}.tgame`, [GAME_FILES], (dir) => games.export(gameId, dir));
		if (path) toasts.success(layout.desktop ? $_("gameUi.exportedPathToast", { values: { path } }) : $_("gameUi.exportedToast"));
	}
	async function removeGame() {
		const title = game?.project.title ?? $_("gameUi.gameFallback");
		if (!(await confirmAction({ title: $_("gameUi.deleteTitle"), message: $_("gameUi.deleteMsg", { values: { title } }), confirmLabel: $_("common.delete"), danger: true }))) return;
		await games.remove(gameId);
		void nav.replace({ page: "game" });
	}
	async function revalidate() {
		const issues = await games.validate();
		if (games.opened) games.opened = { ...games.opened, diagnostics: issues };
		toasts.success(issues.length ? $_("gameUi.issuesFound", { values: { count: issues.length } }) : $_("gameUi.noIssues"));
	}
</script>

{#if loading}
	<div class="center"><Spinner /></div>
{:else if failed || !game}
	<EmptyState title={$_("gameUi.openErrTitle")} description={$_("gameUi.openErrDesc")}>
		{#snippet action()}<Button onclick={() => nav.replace({ page: "game" })}>{$_("gameUi.toList")}</Button>{/snippet}
	</EmptyState>
{:else}
	<div class="detail">
		<div class="banner" class:has-cover={!!cover} class:no-cover={!cover} style:background-image={cover ? `url("${cover}")` : undefined}>
			<div class="cover-tools" class:on={!cover}>
				<Button size="sm" variant="secondary" onclick={() => changeCover()}>{#snippet icon()}<ImagePlus />{/snippet}{cover ? $_("gameUi.changeCover") : $_("gameUi.addCover")}</Button>
				{#if game.project.coverImage}<Button size="sm" variant="secondary" onclick={() => changeCover(true)}>{#snippet icon()}<X />{/snippet}{$_("gameUi.removeCover")}</Button>{/if}
			</div>
		</div>
		<header class="profile" class:no-cover={!cover}>
			<div class="ident">
				<h1>{game.project.title}</h1>
				<p class="faint">{template?.name ?? game.project.templateId} · v{game.project.version}</p>
			</div>
			<div class="cta">
				<Button variant="primary" loading={starting} onclick={() => (games.sessions[0] ? resume(games.sessions[0]) : startNew())}>
					{#snippet icon()}<Play />{/snippet}{games.sessions.length ? $_("gameUi.continuePlay") : $_("gameUi.startPlay")}
				</Button>
				{#if games.sessions.length}<Button variant="ghost" disabled={starting} onclick={startNew}>{#snippet icon()}<Plus />{/snippet}{$_("gameUi.newPlay")}</Button>{/if}
				<Button onclick={() => nav.go({ page: "game", gameId, edit: true })}>{#snippet icon()}<Pencil />{/snippet}{$_("gameUi.editBtn")}</Button>
				<IconButton
					label={$_("gameUi.moreAria")}
					onclick={(e) => {
						const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
						more = { x: r.right, y: r.bottom + 4, align: "end" };
					}}><Ellipsis /></IconButton
				>
			</div>
		</header>
		<ContextMenu at={more} items={moreItems} label={$_("gameUi.menuAria")} onclose={() => (more = null)} />

		<div class="body">
			{#if game.project.description}<p class="intro">{game.project.description}</p>{/if}
			{#if errors.length}
				<button class="alert" onclick={() => (tab = "check")}><AlertTriangle />{$_("gameUi.errorBanner", { values: { count: errors.length } })}<ChevronRight /></button>
			{/if}

			<div class="tabs">
				<Segmented
					label={$_("gameUi.tabsAria")}
					bind:value={tab}
					options={[
						{ value: "sessions", label: $_("gameUi.tabSessions", { values: { count: games.sessions.length } }) },
						{ value: "characters", label: $_("gameUi.tabCharacters", { values: { count: view.characters?.length ?? 0 } }) },
						{ value: "check", label: $_("gameUi.tabCheck", { values: { count: game.diagnostics.length } }) },
					]}
				/>
			</div>

			{#if tab === "sessions"}
				{#if games.sessions.length}
					<div class="sub-toolbar">
						<ListToolbar bind:query={sessionQuery} bind:sort={sessionSort} placeholder={$_("gameUi.searchScenePlaceholder")} sorts={SESSION_SORTS} />
					</div>
				{/if}
				<ul class="list">
					{#each filteredSessions as s, i (s.id)}
						<li>
							<button class="row" onclick={() => resume(s)} disabled={starting}>
								<span class="num">{filteredSessions.length - i}</span>
								<span class="t"><b>{sceneName(s.currentSceneId)}</b><small title={absolute(s.updatedAt)}>{$_("gameUi.sessionMeta", { values: { clock: clock(s.gameTime), turns: s.headSeq, when: relative(s.updatedAt) } })}{s.parentSessionId ? $_("gameUi.branchSuffix") : ""}</small></span>
								<ChevronRight />
							</button>
							<IconButton label={$_("gameUi.copyToChatAria")} size="sm" disabled={copying} onclick={() => void copyToChat(s)}><MessageSquare /></IconButton>
							<IconButton label={$_("gameUi.exportJsonlAria")} size="sm" onclick={() => exportSession(s)}><Download /></IconButton>
							<IconButton label={$_("gameUi.deleteSessionTitle")} size="sm" variant="danger" onclick={() => removeSession(s)}><Trash2 /></IconButton>
						</li>
					{:else}
						<li class="none">{games.sessions.length ? $_("gameUi.noResults") : $_("gameUi.noSessions")} {#if !games.sessions.length}· <button class="link" onclick={startNew}>{$_("gameUi.startPlay")}</button>{/if}</li>
					{/each}
				</ul>
			{:else if tab === "characters"}
				{#if (view.characters?.length ?? 0) > 0}
					<div class="sub-toolbar">
						<ListToolbar bind:query={charQuery} bind:sort={charSort} placeholder={$_("gameUi.searchCharPlaceholder")} sorts={CHAR_SORTS} />
					</div>
				{/if}
				<ul class="list">
					{#each filteredCharacters as c (c.id)}
						<li>
							<div class="row static">
								<Avatar name={c.name} src={gameAssetUrl(gameId, c.avatar)} size={36} />
								<span class="t"><b>{c.name}</b><small>{(c.description ?? "").split("\n")[0] || $_("gameUi.noDesc")}</small></span>
							</div>
							<Button size="sm" variant="ghost" onclick={() => copyToCharacter(c.id, c.name)}>{#snippet icon()}<Users />{/snippet}{$_("gameUi.copyToCharBtn")}</Button>
						</li>
					{:else}
						<li class="none">{(view.characters?.length ?? 0) ? $_("gameUi.noResults") : $_("gameUi.noChars")}</li>
					{/each}
				</ul>
			{:else}
				<div class="check-head">
					<span class="faint">{$_("gameUi.checkHint")}</span>
					<Button size="sm" onclick={revalidate}>{$_("gameUi.recheckBtn")}</Button>
				</div>
				<ul class="issues">
					{#each [...errors, ...warnings] as d, i (i)}
						<li><Badge tone={d.severity === "error" ? "danger" : "warning"}>{d.severity === "error" ? $_("gameUi.sevError") : $_("gameUi.sevWarn")}</Badge><span>{d.text}</span></li>
					{:else}
						<li class="none">{$_("gameUi.noIssues")}</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
{/if}

<style>
	.cover-tools {
		position: absolute;
		top: var(--space-3);
		right: var(--space-4);
		display: flex;
		gap: var(--space-2);
		opacity: 0;
		transition: opacity var(--duration-fast) var(--ease);
	}
	.banner:hover .cover-tools,
	.cover-tools:focus-within,
	.cover-tools.on {
		opacity: 1;
	}
	@media (hover: none) {
		.cover-tools {
			opacity: 1;
		}
	}
	.center {
		display: grid;
		place-items: center;
		height: 100%;
	}
	.detail {
		height: 100%;
		overflow: auto;
	}
	.banner {
		position: relative;
		height: 200px;
	}
	.banner.has-cover {
		background-size: cover;
		background-position: center;
		background-repeat: no-repeat;
	}
	.banner.no-cover {
		height: auto;
		max-width: var(--page-medium);
		margin: 0 auto;
		padding: var(--space-4) var(--space-6) 0;
		display: flex;
		justify-content: flex-end;
	}
	.banner.no-cover .cover-tools {
		position: static;
	}
	.profile {
		position: relative;
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: var(--space-4);
		max-width: var(--page-medium);
		margin: -40px auto 0;
		padding: 0 var(--space-6);
	}
	.profile.no-cover {
		margin-top: var(--space-2);
		align-items: center;
	}
	.profile.no-cover .ident {
		padding: 0;
		background: transparent;
		box-shadow: none;
	}
	.sub-toolbar {
		display: flex;
		justify-content: flex-end;
		margin-bottom: var(--space-3);
	}
	.ident {
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-lg);
		background: var(--bg);
		box-shadow: var(--shadow-sm);
	}
	.ident h1 {
		font-size: var(--text-2xl);
	}
	.cta {
		display: flex;
		gap: var(--space-2);
		align-items: center;
	}
	.body {
		max-width: var(--page-medium);
		margin: 0 auto;
		padding: var(--space-5) var(--space-6) var(--space-10);
	}
	.intro {
		margin-bottom: var(--space-4);
		white-space: pre-wrap;
		color: var(--text-muted);
	}
	.alert {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		margin-bottom: var(--space-4);
		padding: var(--space-3) var(--space-4);
		border: 1px solid var(--danger);
		border-radius: var(--radius-md);
		background: var(--danger-soft);
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.alert :global(svg) {
		width: 16px;
		height: 16px;
		color: var(--danger);
	}
	.alert :global(svg:last-child) {
		margin-left: auto;
		color: var(--text-faint);
	}
	.tabs {
		margin-bottom: var(--space-3);
	}
	.list,
	.issues {
		list-style: none;
		margin: 0;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		overflow: hidden;
	}
	.list li {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding-right: var(--space-2);
	}
	.list li + li,
	.issues li + li {
		border-top: 1px solid var(--border);
	}
	.row {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3);
		border: none;
		border-radius: var(--radius-sm);
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.row.static {
		cursor: default;
	}
	.row:not(.static):hover {
		background: var(--surface-hover);
	}
	.row > :global(svg) {
		width: 16px;
		height: 16px;
		color: var(--text-faint);
	}
	.num {
		width: 28px;
		height: 28px;
		display: grid;
		place-items: center;
		border-radius: 50%;
		background: var(--surface-active);
		font-size: var(--text-xs);
		font-weight: 600;
		flex-shrink: 0;
	}
	.t {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.t small {
		color: var(--text-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.none {
		justify-content: center;
		padding: var(--space-6);
		color: var(--text-faint);
	}
	.link {
		margin-left: 4px;
		padding: 0;
		border: none;
		background: none;
		color: var(--accent-text);
		font: inherit;
		cursor: pointer;
	}
	.check-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: var(--space-2);
		font-size: var(--text-sm);
	}
	.issues li {
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
		padding: var(--space-3);
		font-size: var(--text-sm);
	}
	/* 좁은 화면: 제목이 한 줄에 잘리지 않게 버튼 줄은 아래로 내리고, 긴 제목은 줄바꿈 */
	.ident h1 {
		overflow-wrap: anywhere;
	}
	@media (max-width: 900px) {
		.profile {
			flex-direction: column;
			align-items: stretch;
			padding: 0 var(--space-4);
		}
		.ident {
			min-width: 0;
		}
		.cta {
			flex-wrap: wrap;
		}
	}
</style>
