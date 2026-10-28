<script lang="ts">
	import { onMount } from "svelte";
	import { push } from "svelte-spa-router";
	import type { GameProject } from "$/lib/type/game";
	import { gameManagerCommands } from "$/ipc/commands";
	import { open } from "@tauri-apps/plugin-dialog";
	import Button from "$/components/ui/button/Button.svelte";
	import ButtonGroup from "$/components/ui/button/ButtonGroup.svelte";
	import Card from "$/components/ui/card/Card.svelte";
	import CardGrid from "$/components/ui/card/CardGrid.svelte";
	import Header from "$/components/Header.svelte";
	import { handleOpenGameEditor, handleOpenGameDetailPage, handleStartGamePlay } from "$/app/routes";

	let games = $state<GameProject[]>([]);
	let isLoading = $state(true);
	let errorMessage = $state<string | null>(null);

	async function loadGames() {
		try {
			isLoading = true;
			errorMessage = null;
			games = await gameManagerCommands.getGameList();
		} catch (err) {
			console.error("Failed to load game list:", err);
			errorMessage = "게임 목록을 불러오지 못했습니다.";
		} finally {
			isLoading = false;
		}
	}

	// 새로만들기 버튼 클릭 시 (create_game 커맨드 연동 예시)
	async function handleCreateNewGame() {
		const title = prompt("새 게임 프로젝트의 제목을 입력하세요:");
		if (!title) return;

		const newGameId = await gameManagerCommands.createGame({ title, templateId: "default_template" });
		// 생성 후 바로 에디터로 이동
		push(`/game/${newGameId}/editor`);
	}

	async function handleDeleteGame(gameId: string, title: string) {
		if (!confirm(`"${title}" 게임을 정말 삭제하시겠습니까? 관련 데이터가 모두 사라집니다.`)) {
			return;
		}

		await gameManagerCommands.closeGame();
		await gameManagerCommands.deleteGame({ gameId });
		games = games.filter((g) => g.id !== gameId);
	}

	async function closeGame() {
		await gameManagerCommands.closeGame();
	}

	async function handleExport(gameId: string) {
		await gameManagerCommands.exportGame({ gameId, outputDir: "\\download" });
	}
	async function handleImportGame() {
		// 유저가 폴더 선택
		const selected = await open({
			directory: false,
			multiple: false,
		});
		if (!selected) return;
		await gameManagerCommands.importGame({ zipPath: selected });
		loadGames();
	}

	onMount(() => {
		loadGames();
	});
</script>

<Header>
	<div>타이틀</div>
</Header>
<div class="game-list-container">
	<header class="header">
		<div class="title-area">
			<h1>내 게임 라이브러리</h1>
			<p class="subtitle">보유 중인 텍스트 게임 프로젝트 목록입니다.</p>
		</div>
		<ButtonGroup>
			<Button variant="primary" onclick={handleCreateNewGame}>+ 새로만들기</Button>
			<Button variant="primary" onclick={handleImportGame}>불러오기</Button>
		</ButtonGroup>
	</header>

	{#if isLoading}
		<div class="status-box">게임 목록을 불러오는 중...</div>
	{:else if errorMessage}
		<div class="status-box error">{errorMessage}</div>
	{:else if games.length === 0}
		<div class="empty-box">
			<p>등록된 게임이 없습니다.</p>
			<p class="hint">상단의 '+ 새로만들기' 버튼을 눌러 프로젝트를 시작해 보세요.</p>
		</div>
	{:else}
		<CardGrid class="game-grid">
			{#each games as game (game.id)}
				<Card class="game-card" onclick={() => handleOpenGameDetailPage(game.id)}>
					<div class="cover-area">
						{#if game.coverImage}
							<img src={game.coverImage} alt={game.title} class="cover-img" />
						{:else}
							<div class="cover-placeholder">
								<span>{game.title.slice(0, 2)}</span>
							</div>
						{/if}
						<span class="version-badge">v{game.version}</span>
					</div>

					<div class="content-area">
						<h3 class="game-title" title={game.title}>{game.title}</h3>
						<p class="game-desc">
							{game.description || "설명이 작성되지 않은 프로젝트입니다."}
						</p>
						<div class="meta-info">
							<span>수정: {new Date(game.updatedAt).toLocaleDateString()}</span>
						</div>
					</div>

					<ButtonGroup class="action-footer">
						<Button
							variant="primary"
							onclick={(e) => {
								e.stopPropagation();
								handleStartGamePlay(game.id);
							}}>시작</Button
						>
						<Button
							onclick={(e) => {
								e.stopPropagation();
								handleOpenGameEditor(game.id);
							}}>편집</Button
						>
						<Button
							onclick={(e) => {
								e.stopPropagation();
								handleExport(game.id);
							}}>내보내기</Button
						>
						<Button
							variant="danger"
							onclick={(e) => {
								e.stopPropagation();
								handleDeleteGame(game.id, game.title);
							}}>삭제</Button
						>
					</ButtonGroup>
				</Card>
			{/each}
		</CardGrid>
	{/if}
</div>

<style>
	.game-list-container {
		padding: 2rem;
		max-width: 1200px;
		margin: 0 auto;
	}
	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
		border-bottom: 1px solid var(--border-color);
		padding-bottom: 1rem;
	}
	.title-area h1 {
		font-size: 1.8rem;
		font-weight: 700;
		margin: 0 0 0.4rem 0;
	}
	.subtitle {
		color: var(--text-muted);
		font-size: 0.95rem;
		margin: 0;
	}

	.status-box,
	.empty-box {
		text-align: center;
		padding: 4rem 2rem;
		color: #94a3b8;
		background: #1e293b;
		border-radius: 12px;
		border: 1px dashed var(--border-color);
	}
	.status-box.error {
		color: #f87171;
	}
	.empty-box .hint {
		font-size: 0.85rem;
		margin-top: 0.5rem;
	}
	.cover-area {
		position: relative;
		height: 140px;
		background: #0f172a;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.cover-img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.cover-placeholder {
		font-size: 2rem;
		font-weight: 700;
		color: var(--border-strong-color);
		background: #111827;
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.version-badge {
		position: absolute;
		top: 10px;
		right: 10px;
		background: rgba(15, 23, 42, 0.8);
		backdrop-filter: blur(4px);
		color: #38bdf8;
		padding: 0.2rem 0.5rem;
		font-size: 0.75rem;
		font-weight: 600;
		border-radius: 4px;
		border: 1px solid var(--border-color);
	}
	.content-area {
		padding: 1.25rem;
		flex: 1;
		display: flex;
		flex-direction: column;
	}
	.game-title {
		font-size: 1.15rem;
		font-weight: 600;
		margin: 0 0 0.5rem 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.game-desc {
		font-size: 0.875rem;
		color: var(--text-muted);
		margin: 0 0 1rem 0;
		line-height: 1.4;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		flex: 1;
	}
	.meta-info {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
</style>
