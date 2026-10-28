<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { pop, push } from "svelte-spa-router";
	import { gameEngineCommands } from "$/ipc/commands";
	import type { GamePlayerSessions } from "$/lib/type/game";

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
			const res = await gameEngineCommands.startNewGame();
			// 세션 플레이 화면으로 이동 (예: /play/:sessionId)
			push(`/game/${params.gameId}/play/${res.state.sessionId}`);
		} catch (err) {
			console.error("Failed to start new game session:", err);
			alert("새 세션을 시작하는 중 오류가 발생했습니다.");
		}
	}

	// 기존 세션 이어하기 (continue_game)
	async function handleContinueSession(sessionId: string) {
		try {
			await gameEngineCommands.continueGame({sessionId: sessionId});
			push(`/game/${params.gameId}/play/${sessionId}`);
		} catch (err) {
			console.error("Failed to continue session:", err);
			alert("세션을 불러오지 못했습니다.");
		}
	}

	function handleBack() {
		pop();
	}

	onMount(() => {
		loadSessions();
	});
</script>

<div class="session-list-container">
	<header class="header">
		<div class="title-area">
			<button class="back-btn" onclick={handleBack}>← 돌아가기</button>
			<h1>게임 세션 선택</h1>
		</div>
		<button class="primary-btn" onclick={handleStartNewSession}>+ 새 게임 시작</button>
	</header>

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
						<h3>세션 ID: {session.id}</h3>
						<span class="date">최근 플레이: {new Date(session.updatedAt).toLocaleString()}</span>
					</div>
					<button class="continue-btn" onclick={() => handleContinueSession(session.id)}>
						이어하기 →
					</button>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.session-list-container {
		padding: 2rem;
		max-width: 900px;
		margin: 0 auto;
		color: #e2e8f0;
		font-family: inherit;
	}
	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
		border-bottom: 1px solid #334155;
		padding-bottom: 1rem;
	}
	.title-area {
		display: flex;
		align-items: center;
		gap: 1rem;
	}
	.title-area h1 {
		font-size: 1.6rem;
		font-weight: 700;
		margin: 0;
	}
	.back-btn {
		background: #334155;
		color: #e2e8f0;
		border: none;
		padding: 0.5rem 0.8rem;
		border-radius: 6px;
		cursor: pointer;
		font-weight: 600;
	}
	.back-btn:hover {
		background: #475569;
	}

	.primary-btn {
		background: #10b981;
		color: white;
		border: none;
		padding: 0.6rem 1.2rem;
		border-radius: 6px;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.2s;
	}
	.primary-btn:hover {
		background: #059669;
	}

	.status-box,
	.empty-box {
		text-align: center;
		padding: 4rem 2rem;
		color: #94a3b8;
		background: #1e293b;
		border-radius: 12px;
		border: 1px dashed #334155;
	}
	.status-box.error {
		color: #f87171;
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
		background: #1e293b;
		border-radius: 8px;
		border: 1px solid #334155;
		padding: 1.2rem 1.5rem;
		display: flex;
		justify-content: space-between;
		align-items: center;
		transition: border-color 0.2s;
	}
	.session-card:hover {
		border-color: #38bdf8;
	}

	.session-info {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.session-info h3 {
		margin: 0;
		font-size: 1.05rem;
		color: #f8fafc;
		font-family: monospace;
	}
	.session-info .date {
		font-size: 0.8rem;
		color: #94a3b8;
	}

	.continue-btn {
		background: #3b82f6;
		color: white;
		border: none;
		padding: 0.5rem 1rem;
		border-radius: 6px;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.2s;
	}
	.continue-btn:hover {
		background: #2563eb;
	}
</style>
