<script lang="ts">
	import { _ } from "svelte-i18n";
	import { onMount } from "svelte";
	import type { GameProject, OpenGameResponse } from "$/lib/type/game";
	import { gameEngineCommands, gameManagerCommands } from "$/ipc/commands";
	import Header from "$/components/Header.svelte";
	import Icon from "$/components/ui/Icon.svelte";
	import { handleGoBack } from "$/app/routes";
	import IconButton from "$/components/ui/button/IconButton.svelte";
	import { handleOpenGameEditor, handleStartGamePlay } from "$/lib/game/gameStore.svelte";

	type Props = { params: { gameId: string } };
	// props로 라우터 파라미터 전달받음 (svelte-spa-router 규격)
	let { params }: Props = $props();

	let game = $state<OpenGameResponse | null>(null);
	let isLoading = $state(true);
	let errorMessage = $state<string | null>(null);

	// 게임 상세 정보(Manifest) 불러오기 (open_game 또는 get_manifest 커맨드 연동)
	async function loadGameDetail() {
		try {
			isLoading = true;
			errorMessage = null;
			// open_game 커맨드를 통해 해당 게임 메타데이터 로드
			game = await gameManagerCommands.openGame({ gameId: params.gameId });
		} catch (err) {
			console.error("Failed to load game detail:", err);
			errorMessage = "게임 정보를 불러오지 못했습니다.";
		} finally {
			isLoading = false;
		}
	}

	onMount(() => {
		loadGameDetail();
	});
</script>

<Header>
	<div class="header-left">
		<IconButton size="lg" aria-label={$_("global.back")} onclick={handleGoBack}>
			<Icon name={"ChevronLeft"} />
		</IconButton>
	</div>
</Header>
<div class="content detail-container">
	{#if isLoading}
		<div class="status-box">게임 정보를 불러오는 중...</div>
	{:else if errorMessage}
		<div class="status-box error">{errorMessage}</div>
	{:else if game}
		<!-- 상단 커버 이미지 영역 -->
		<div class="cover-hero">
			{#if game.project.coverImage}
				<img src={game.project.coverImage} alt={game.project.title} class="cover-image" />
			{:else}
				<div class="cover-placeholder">
					<span>{game.project.title}</span>
				</div>
			{/if}
			<div class="hero-overlay">
				<div class="hero-titles">
					<span class="version-badge">v{game.project.version}</span>
					<h1>{game.project.title}</h1>
				</div>
				<div class="hero-actions">
					<button class="action-btn play" onclick={() => handleStartGamePlay(params.gameId)}
						>게임 플레이</button
					>
					<button class="action-btn edit" onclick={() => handleOpenGameEditor(params.gameId)}
						>프로젝트 편집</button
					>
				</div>
			</div>
		</div>

		<!-- 하단 게임 상세 정보 영역 -->
		<section class="info-section">
			<div class="info-card description-box">
				<h3>게임 소개</h3>
				<p>{game.project.description || "등록된 게임 설명이 없습니다."}</p>
			</div>

			<div class="info-card specs-box">
				<h3>상세 메타데이터</h3>
				<ul class="specs-list">
					<li><span>프로젝트 ID</span> <code>{game.project.id}</code></li>
					<li><span>템플릿 ID</span> <code>{game.project.templateId}</code></li>
					<li><span>엔진 API 버전</span> <code>{game.project.apiVersion}</code></li>
					<li><span>초기 위치 ID</span> <code>{game.project.startLocationId || "설정되지 않음"}</code></li>
					<li><span>생성일</span> {new Date(game.project.createdAt).toLocaleString()}</li>
					<li><span>최종 수정일</span> {new Date(game.project.updatedAt).toLocaleString()}</li>
				</ul>
			</div>
		</section>
	{/if}
</div>

<style>
	.detail-container {
		display: flex;
		flex-direction: column;
		background: #0b0f19;
		height: calc(100vh - var(--header-height));
	}

	.status-box {
		text-align: center;
		padding: 6rem 2rem;
		color: #9ca3af;
		font-size: 1.1rem;
	}
	.status-box.error {
		color: #f87171;
	}

	/* 커버 이미지 영역 */
	.cover-hero {
		position: relative;
		width: 100%;
		height: 320px;
		background: #111827;
		overflow: hidden;
		display: flex;
		align-items: flex-end;
	}
	.cover-image {
		width: 100%;
		height: 100%;
		object-fit: cover;
		filter: brightness(0.7);
	}
	.cover-placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, #1f2937, #111827);
		font-size: 2.5rem;
		font-weight: 700;
		color: #4b5563;
	}

	.hero-overlay {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		padding: 2rem;
		background: linear-gradient(to top, rgba(11, 15, 25, 0.95), rgba(11, 15, 25, 0));
		display: flex;
		justify-content: space-between;
		align-items: flex-end;
	}
	.hero-titles {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.hero-titles h1 {
		margin: 0;
		font-size: 2.2rem;
		font-weight: 700;
		color: #f9fafb;
		text-shadow: 0 2px 4px rgba(0, 0, 0, 0.5);
	}

	.version-badge {
		align-self: flex-start;
		background: #38bdf8;
		color: #0b0f19;
		font-size: 0.75rem;
		font-weight: 700;
		padding: 0.2rem 0.6rem;
		border-radius: 4px;
	}

	.hero-actions {
		display: flex;
		gap: 0.8rem;
	}
	.action-btn {
		padding: 0.7rem 1.4rem;
		border-radius: 8px;
		font-weight: 600;
		font-size: 0.95rem;
		cursor: pointer;
		border: none;
		transition:
			transform 0.15s,
			background 0.15s;
	}
	.action-btn:hover {
		transform: translateY(-2px);
	}
	.action-btn.play {
		background: #10b981;
		color: white;
	}
	.action-btn.play:hover {
		background: #059669;
	}
	.action-btn.edit {
		background: #374151;
		color: white;
		border: 1px solid #4b5563;
	}
	.action-btn.edit:hover {
		background: #4b5563;
	}

	/* 하단 정보 영역 */
	.info-section {
		max-width: 1000px;
		width: 100%;
		margin: 0 auto;
		padding: 2.5rem 2rem;
		display: grid;
		grid-template-columns: 2fr 1fr;
		gap: 2rem;
	}
	.info-card {
		background: #111827;
		border: 1px solid #1f2937;
		border-radius: 12px;
		padding: 1.5rem;
	}
	.info-card h3 {
		margin: 0 0 1rem 0;
		font-size: 1.1rem;
		color: #f3f4f6;
		border-bottom: 1px solid #1f2937;
		padding-bottom: 0.6rem;
	}

	.description-box p {
		margin: 0;
		line-height: 1.6;
		color: #9ca3af;
		font-size: 0.95rem;
		white-space: pre-wrap;
	}

	.specs-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.8rem;
		font-size: 0.9rem;
	}
	.specs-list li {
		display: flex;
		justify-content: space-between;
		align-items: center;
		color: #9ca3af;
	}
	.specs-list code {
		color: #38bdf8;
		background: #030712;
		padding: 0.1rem 0.4rem;
		border-radius: 4px;
		font-family: monospace;
		font-size: 0.85rem;
	}
</style>
