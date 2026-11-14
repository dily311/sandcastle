<script lang="ts">
	import { _ } from "svelte-i18n";
	import { onMount } from "svelte";
	import { pop, push } from "svelte-spa-router";
	import { gameEngineCommands } from "$/ipc/commands";
	import type { GamePlayerSessions } from "$/lib/type/game";
	import { handleGoBack } from "$/app/routes";
	import IconButton from "$/components/ui/button/IconButton.svelte";
	import Icon from "$/components/ui/Icon.svelte";
	import Button from "$/components/ui/button/Button.svelte";
	import Header from "$/components/Header.svelte";
	import ButtonGroup from "$/components/ui/button/ButtonGroup.svelte";
	import { newGameSession, startGameSession } from "$/lib/game/services/gameSessionService";

	type Props = {
		params: { gameId: string };
	};
	// 라우터 파라미터로 gameId를 전달받음 (예: /game/:gameId/sessions)
	let { params }: Props = $props();

	let sessions = $state<GamePlayerSessions[]>([]);
	let isLoading = $state(true);
	let errorMessage = $state<string | null>(null);

	// 세션 목록 불러오기 (list_sessions)
	async function loadSessions() {
		try {
			isLoading = true;
			errorMessage = null;
			sessions = await gameEngineCommands.listSessions();
		} catch (err) {
			console.error("Failed to load sessions:", err);
			errorMessage = "세션 목록을 불러오지 못했습니다.";
		} finally {
			isLoading = false;
		}
	}

	// 새 게임/세션 시작 (start_new_game)
	async function handleStartNewSession() {
		try {
			// 필요에 따라 gameId를 전달할 수도 있음
			const sessionId = await newGameSession(params.gameId);
			// 세션 플레이 화면으로 이동 (예: /play/:sessionId)
			push(`/game/${params.gameId}/play/${sessionId}`);
		} catch (err) {
			console.error("Failed to start new game session:", err);
			alert("새 세션을 시작하는 중 오류가 발생했습니다.");
		}
	}

	// 기존 세션 이어하기 (continue_game)
	async function handleContinueSession(sessionId: string) {
		try {
			await startGameSession(params.gameId, sessionId);
			push(`/game/${params.gameId}/play/${sessionId}`);
		} catch (err) {
			console.error("Failed to continue session:", err);
			alert("세션을 불러오지 못했습니다.");
		}
	}

	async function handleDeleteSession(sessionId: string) {
		await gameEngineCommands.deleteSessions({ sessionId });
		sessions = sessions.filter((s) => s.id !== sessionId);
	}

	onMount(() => {
		loadSessions();
	});
</script>

<Header class="header">
	<div class="title-area">
		<IconButton aria-label={$_("global.back")} onclick={handleGoBack}>
			<Icon name={"ChevronLeft"} />
		</IconButton>
		<h1>게임 세션 선택</h1>
	</div>
	<Button variant="primary" onclick={handleStartNewSession}>
		<Icon name={"plus"} />
		새 게임 시작
	</Button>
</Header>
<div class="page">
	<div class="session-list-container">
		{#if isLoading}
			<div class="status-box">저장된 세션을 불러오는 중...</div>
		{:else if errorMessage}
			<div class="status-box error">{errorMessage}</div>
		{:else if sessions.length === 0}
			<div class="empty-box">
				<p>진행 중인 세션이 없습니다.</p>
				<p class="hint">상단의 '+ 새 게임 시작' 버튼을 눌러 모험을 시작하세요.</p>
			</div>
		{:else}
			<div class="session-grid">
				{#each sessions as session (session.id)}
					<div class="session-card">
						<div class="session-info">
							<h3>저장된 게임</h3>
							<span class="date">최근 플레이: {new Date(session.updatedAt).toLocaleString()}</span>
						</div>
						<ButtonGroup>
							<Button variant="secondary" onclick={() => handleContinueSession(session.id)}>
								이어하기 →
							</Button>
							<Button
								variant="danger"
								onclick={(e) => {
									e.stopPropagation();
									handleDeleteSession(session.id);
								}}
							>
								삭제
							</Button>
						</ButtonGroup>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	.session-list-container {
		padding: 2rem;
		max-width: 900px;
		margin: 0 auto;
		overflow: hidden auto;
	}

	.title-area {
		display: flex;
		align-items: center;
		gap: 1rem;
	}
	.title-area h1 {
		font-size: 1rem;
		font-weight: 500;
		margin: 0;
	}

	.status-box,
	.empty-box {
		text-align: center;
		padding: 4rem 2rem;
		color: var(--text-muted);
		background: var(--card-bg);
		border-radius: 12px;
		border: 1px dashed var(--card-border-color);
	}
	.status-box.error {
		color: var(--color-red);
	}
	.empty-box .hint {
		font-size: 0.85rem;
		margin-top: 0.5rem;
	}

	.session-grid {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.session-card {
		background: var(--card-bg);
		border-radius: 8px;
		border: 1px solid var(--card-border-color);
		padding: 1.2rem 1.5rem;
		display: flex;
		justify-content: space-between;
		align-items: center;
		transition: border-color 0.2s;
	}
	.session-card:hover {
		border-color: var(--card-border-hover-color);
	}

	.session-info {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.session-info h3 {
		margin: 0;
		font-size: 1.05rem;
		color: var(--text-color);
	}
	.session-info .date {
		font-size: 0.8rem;
		color: var(--text-muted);
	}
</style>
