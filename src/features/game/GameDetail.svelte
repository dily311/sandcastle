<script lang="ts">
	import { onMount } from "svelte";
	import { AlertTriangle, ImagePlus, X, ChevronRight, Download, Ellipsis, Gamepad2, Pencil, Play, Plus, Trash2, Users } from "@lucide/svelte";
	import { Avatar, Badge, Button, ContextMenu, EmptyState, IconButton, Segmented, Spinner, type MenuItem } from "$shared/ui";
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
	$effect(() => dropPaths({ label: "게임 커버로 쓰기", hint: "PNG·JPG·WEBP·GIF 한 장", multiple: false, onpaths: ([path]) => setCover(path) }));

	let tab = $state<"sessions" | "characters" | "check">("sessions");
	let starting = $state(false);

	async function startNew() {
		if (errors.length) {
			tab = "check";
			return toasts.error("데이터 오류가 있어 시작할 수 없음. 진단 탭 확인");
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
			toasts.error("세션을 불러오지 못함", String((e as Error)?.message ?? e));
		} finally {
			starting = false;
		}
	}
	async function removeSession(s: Session) {
		if (await confirmAction({ title: "세션 삭제", message: `${relative(s.updatedAt)}에 플레이한 기록을 휴지통으로 옮김`, confirmLabel: "삭제", danger: true })) await games.removeSession(s.id);
	}
	const sceneName = (id: string | null) => view.scenes?.find((x) => x.id === id)?.name ?? view.scenes?.find((x) => x.id === id)?.sceneKey ?? "처음";
	const clock = (sec = 0) => {
		const m = Math.floor(sec / 60);
		return `${Math.floor(m / 60) % 24}:${String(m % 60).padStart(2, "0")}`;
	};

	/** 플레이 기록 → SillyTavern 채팅 (5-E-20-3). 연출 태그는 걷고 선택지는 줄로 */
	async function exportSession(s: Session) {
		const title = `${game?.project.title ?? "game"} - ${sceneName(s.currentSceneId)}`;
		const n = await saveTo(`${title}.jsonl`, [{ name: "SillyTavern 채팅", extensions: ["jsonl"] }], (path) => call(commands.exportGameSession(s.id, path), { source: "game" }));
		if (n != null) toasts.success(`메시지 ${n}개 내보냄`);
	}

	async function copyToCharacter(id: string, name: string) {
		const c = await call(commands.copyGameCharacterToCharacter(id), { source: "game" });
		library.characters = [...library.characters, c];
		toasts.show({ tone: "success", message: `캐릭터 '${name}' 만듦`, action: { label: "열기", run: () => nav.go({ page: "character", charId: c.id }) } });
	}

	let more = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	const moreItems: MenuItem[] = [
		{ label: layout.desktop ? "내보내기 (다운로드 폴더)" : "내보내기", icon: Download, onselect: exportGame },
		{ label: "삭제", icon: Trash2, danger: true, separator: true, onselect: removeGame },
	];
	async function exportGame() {
		// Android는 공용 다운로드 폴더에 직접 쓸 수 없어 저장 위치를 고른다
		const path = layout.desktop ? await games.export(gameId) : await saveIntoPicked(`${game?.project.title ?? "game"}.tgame`, [GAME_FILES], (dir) => games.export(gameId, dir));
		if (path) toasts.success(layout.desktop ? `내보냄 · ${path}` : "내보냄");
	}
	async function removeGame() {
		const title = game?.project.title ?? "게임";
		if (!(await confirmAction({ title: "게임 삭제", message: `'${title}'과 모든 플레이 기록을 휴지통으로 옮김. 설정 > 데이터 > 휴지통에서 되살릴 수 있음`, confirmLabel: "삭제", danger: true }))) return;
		await games.remove(gameId);
		void nav.replace({ page: "game" });
	}
	async function revalidate() {
		const issues = await games.validate();
		if (games.opened) games.opened = { ...games.opened, diagnostics: issues };
		toasts.success(issues.length ? `문제 ${issues.length}개` : "문제 없음");
	}
</script>

{#if loading}
	<div class="center"><Spinner /></div>
{:else if failed || !game}
	<EmptyState title="게임을 열 수 없음" description="지워졌거나 파일이 깨졌을 수 있음">
		{#snippet action()}<Button onclick={() => nav.replace({ page: "game" })}>목록으로</Button>{/snippet}
	</EmptyState>
{:else}
	<div class="detail">
		<div class="banner" style:background-image={cover ? `url("${cover}")` : undefined}>
			<div class="cover-tools">
				<Button size="sm" variant="secondary" onclick={() => changeCover()}>{#snippet icon()}<ImagePlus />{/snippet}{cover ? "커버 바꾸기" : "커버 넣기"}</Button>
				{#if game.project.coverImage}<Button size="sm" variant="secondary" onclick={() => changeCover(true)}>{#snippet icon()}<X />{/snippet}커버 지우기</Button>{/if}
			</div>
		</div>
		<header class="profile">
			<div class="ident">
				<h1>{game.project.title}</h1>
				<p class="faint">{template?.name ?? game.project.templateId} · v{game.project.version}</p>
			</div>
			<div class="cta">
				<Button variant="primary" loading={starting} onclick={() => (games.sessions[0] ? resume(games.sessions[0]) : startNew())}>
					{#snippet icon()}<Play />{/snippet}{games.sessions.length ? "이어하기" : "시작"}
				</Button>
				{#if games.sessions.length}<Button variant="ghost" disabled={starting} onclick={startNew}>{#snippet icon()}<Plus />{/snippet}새로 시작</Button>{/if}
				<Button onclick={() => nav.go({ page: "game", gameId, edit: true })}>{#snippet icon()}<Pencil />{/snippet}편집</Button>
				<IconButton
					label="더 보기"
					onclick={(e) => {
						const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
						more = { x: r.right, y: r.bottom + 4, align: "end" };
					}}><Ellipsis /></IconButton
				>
			</div>
		</header>
		<ContextMenu at={more} items={moreItems} label="게임 메뉴" onclose={() => (more = null)} />

		<div class="body">
			{#if game.project.description}<p class="intro">{game.project.description}</p>{/if}
			{#if errors.length}
				<button class="alert" onclick={() => (tab = "check")}><AlertTriangle />데이터 오류 {errors.length}개 · 고쳐야 시작할 수 있음<ChevronRight /></button>
			{/if}

			<div class="tabs">
				<Segmented
					label="게임 탭"
					bind:value={tab}
					options={[
						{ value: "sessions", label: `플레이 기록 ${games.sessions.length}` },
						{ value: "characters", label: `등장인물 ${view.characters?.length ?? 0}` },
						{ value: "check", label: `진단 ${game.diagnostics.length}` },
					]}
				/>
			</div>

			{#if tab === "sessions"}
				<ul class="list">
					{#each games.sessions as s, i (s.id)}
						<li>
							<button class="row" onclick={() => resume(s)} disabled={starting}>
								<span class="num">{games.sessions.length - i}</span>
								<span class="t"><b>{sceneName(s.currentSceneId)}</b><small title={absolute(s.updatedAt)}>게임 시각 {clock(s.gameTime)} · {s.headSeq}턴 · {relative(s.updatedAt)}{s.parentSessionId ? " · 갈래" : ""}</small></span>
								<ChevronRight />
							</button>
							<IconButton label="SillyTavern 채팅(.jsonl)으로 내보내기" size="sm" onclick={() => exportSession(s)}><Download /></IconButton>
							<IconButton label="세션 삭제" size="sm" onclick={() => removeSession(s)}><Trash2 /></IconButton>
						</li>
					{:else}
						<li class="none">아직 플레이 기록 없음 · <button class="link" onclick={startNew}>시작</button></li>
					{/each}
				</ul>
			{:else if tab === "characters"}
				<ul class="list">
					{#each view.characters ?? [] as c (c.id)}
						<li>
							<div class="row static">
								<Avatar name={c.name} src={gameAssetUrl(gameId, c.avatar)} size={36} />
								<span class="t"><b>{c.name}</b><small>{(c.description ?? "").split("\n")[0] || "설명 없음"}</small></span>
							</div>
							<Button size="sm" variant="ghost" onclick={() => copyToCharacter(c.id, c.name)}>{#snippet icon()}<Users />{/snippet}캐릭터로 복사</Button>
						</li>
					{:else}
						<li class="none">등장인물 없음</li>
					{/each}
				</ul>
			{:else}
				<div class="check-head">
					<span class="faint">게임 데이터 검사 결과. 오류가 있으면 새로 시작할 수 없음</span>
					<Button size="sm" onclick={revalidate}>다시 검사</Button>
				</div>
				<ul class="issues">
					{#each [...errors, ...warnings] as d, i (i)}
						<li><Badge tone={d.severity === "error" ? "danger" : "warning"}>{d.severity === "error" ? "오류" : "경고"}</Badge><span>{d.text}</span></li>
					{:else}
						<li class="none">문제 없음</li>
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
	.cover-tools:focus-within {
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
		background:
			radial-gradient(ellipse at 70% 0%, color-mix(in oklab, var(--accent) 45%, transparent), transparent 55%),
			linear-gradient(180deg, var(--gray-4), var(--gray-2));
		background-size: cover;
		background-position: center;
		z-index: -1;
	}
	.profile {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: var(--space-4);
		max-width: var(--page-medium);
		margin: -40px auto 0;
		padding: 0 var(--space-6);
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
		padding: var(--space-1);
		border-radius: var(--radius-lg);
		background: var(--bg-sunken);
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
