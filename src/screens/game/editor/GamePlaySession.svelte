<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { push } from "svelte-spa-router";
	import type { GameEvent, GameJournal, GameResponse, NpcReply, RenderState } from "$/lib/type/game";
	import { gameEngineCommands } from "$/ipc/commands";

	// 라우터 파라미터로 세션 ID 또는 게임 ID를 받음
	type Props = { params: { sessionId: string } };
	let { params }: Props = $props();

	// 상태 관리
	let renderState = $state<RenderState | null>(null);
	let eventLogs = $state<GameEvent[]>([]);
	let journals = $state<GameJournal[]>([]);

	let playerCommand = $state("");
	let npcInput = $state("");
	let npcLog = $state<{ sender: string; text: string }[]>([]);
	let isLoading = $state(false);

	// 1. 세션 연결 및 초기 상태 복원 (continue_game 또는 load)
	async function initSession() {
		try {
			isLoading = true;
			// 세션 이어하기 호출 시 초기 GameResponse나 상태를 반환받는다고 가정
			const res = await gameEngineCommands.continueGame({ sessionId: params.sessionId });
			if (res) {
				renderState = res.state;
				eventLogs = res.events || [];
			}
			// 저널 로그 불러오기
			await loadJournals();
		} catch (err) {
			console.error("Failed to initialize session:", err);
			alert("세션을 불러오지 못했습니다.");
		} finally {
			isLoading = false;
		}
	}

	// 2. 저널 읽기 (read_journal)
	async function loadJournals() {
		try {
			const history = await invoke<GameJournal[]>("read_journal", {
				session_id: params.sessionId,
				to_seq: 100, // 최근 100개 기준 (필요에 따라 조절)
			});
			journals = history;
		} catch (err) {
			console.error("Failed to read journal:", err);
		}
	}

	// 3. 인게임 명령어 디스패치 (engine -> 새 RenderState 및 events 반환)
	async function handleDispatchCommand(e: SubmitEvent) {
		e.preventDefault();
		if (!playerCommand.trim() || isLoading) return;

		const cmd = playerCommand;
		playerCommand = "";

		try {
			isLoading = true;
			const res = await invoke<GameResponse>("dispatch_game_command", {
				command: { sessionId: params.sessionId, type: "action", payload: cmd },
			});

			// 엔진에서 처리된 결과(RenderState와 발생한 이벤트들) 반영
			renderState = res.state;
			if (res.events && res.events.length > 0) {
				eventLogs = [...res.events, ...eventLogs];
			}

			// 저널 갱신
			await loadJournals();
		} catch (err) {
			console.error("Command dispatch failed:", err);
		} finally {
			isLoading = false;
		}
	}

	// 4. NPC 소통 (npc_say -> NpcReply 반환)
	async function handleNpcSay(e: SubmitEvent) {
		e.preventDefault();
		if (!npcInput.trim() || !renderState?.activeNpcId) return;

		const query = npcInput;
		npcInput = "";
		npcLog = [...npcLog, { sender: "플레이어", text: query }];

		try {
			const reply = await invoke<NpcReply>("npc_say", {
				character_id: renderState.activeNpcId,
				input: query,
			});

			// NPC 응답 텍스트 및 엔진 계산 상태/이벤트 반영
			npcLog = [...npcLog, { sender: "NPC", text: reply.text }];
			renderState = reply.state;
			if (reply.events) {
				eventLogs = [...reply.events, ...eventLogs];
			}
		} catch (err) {
			console.error("NPC communication failed:", err);
			npcLog = [...npcLog, { sender: "시스템", text: "NPC와 소통하지 못했습니다." }];
		}
	}

	function handleBack() {
		push(`/game/${params.sessionId}/sessions`); // 세션 리스트로 복귀
	}

	onMount(() => {
		initSession();
	});
</script>

<div class="session-play-layout">
	<!-- 상단 대시보드 (RenderState 정보 바) -->
	<header class="state-dashboard">
		<button class="back-btn" onclick={handleBack}>← 세션 목록</button>
		{#if renderState}
			<div class="metrics">
				<span>📍 맵: <code>{renderState.mapId}</code></span>
				<span>🚩 위치: <code>{renderState.locationId || "없음"}</code></span>
				<span>⏰ 시간: <strong>{renderState.gameTime}분</strong> ({renderState.timePeriodId})</span>
				<span>⛅ 날씨: <strong>{renderState.weatherID}</strong></span>
				<span>💬 활성 NPC: <strong>{renderState.activeNpcId || "없음"}</strong></span>
			</div>
		{:else}
			<div class="metrics">상태 동기화 중...</div>
		{/if}
	</header>

	<div class="play-main-grid">
		<!-- 좌측: 이벤트 및 저널 로그 (엔진 결과 출력) -->
		<section class="log-section">
			<h3>게임 저널 & 이벤트 로그</h3>
			<div class="log-box">
				{#each journals as journal}
					<div class="log-item">
						<span class="time">[{journal.gameTime}분]</span>
						<span class="kind">({journal.kind})</span>
						<span class="payload">{JSON.stringify(journal.payloadJson)}</span>
					</div>
				{:else}
					<div class="empty-log">기록된 저널이 없습니다.</div>
				{/each}
			</div>

			<!-- 명령어 입력 Form -->
			<form class="command-form" onsubmit={handleDispatchCommand}>
				<input
					type="text"
					bind:value={playerCommand}
					placeholder="명령어 입력 (예: move north, inspect room)..."
					disabled={isLoading}
				/>
				<button type="submit" disabled={isLoading}>실행</button>
			</form>
		</section>

		<!-- 우측: NPC 대화 패널 -->
		<section class="npc-interaction-section">
			<h3>NPC 대화 (엔진 처리)</h3>
			<div class="npc-chat-container">
				{#each npcLog as chat}
					<div class="chat-bubble {chat.sender === '플레이어' ? 'me' : 'npc'}">
						<strong>{chat.sender}:</strong>
						{chat.text}
					</div>
				{:else}
					<div class="empty-log">현재 상호작용 중인 NPC가 대기 중입니다.</div>
				{/each}
			</div>

			<form class="npc-form" onsubmit={handleNpcSay}>
				<input
					type="text"
					bind:value={npcInput}
					placeholder={renderState?.activeNpcId ? "NPC에게 말을 겁니다..." : "활성화된 NPC가 없습니다."}
					disabled={!renderState?.activeNpcId}
				/>
				<button type="submit" disabled={!renderState?.activeNpcId}>전송</button>
			</form>
		</section>
	</div>
</div>

<style>
	.session-play-layout {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background: #090d16;
		color: #e2e8f0;
		font-family: inherit;
	}

	.state-dashboard {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.8rem 1.5rem;
		background: #111827;
		border-bottom: 1px solid #1f2937;
	}
	.back-btn {
		background: #374151;
		color: #f3f4f6;
		border: none;
		padding: 0.4rem 0.8rem;
		border-radius: 6px;
		cursor: pointer;
		font-size: 0.85rem;
		font-weight: 600;
	}
	.back-btn:hover {
		background: #4b5563;
	}

	.metrics {
		display: flex;
		gap: 1.2rem;
		font-size: 0.85rem;
		color: #9ca3af;
	}
	.metrics code {
		color: #38bdf8;
		background: #030712;
		padding: 0.1rem 0.3rem;
		border-radius: 4px;
	}
	.metrics strong {
		color: #f3f4f6;
	}

	.play-main-grid {
		display: grid;
		grid-template-columns: 3fr 2fr;
		gap: 1.5rem;
		flex: 1;
		padding: 1.5rem;
		overflow: hidden;
	}

	.log-section,
	.npc-interaction-section {
		background: #111827;
		border: 1px solid #1f2937;
		border-radius: 10px;
		display: flex;
		flex-direction: column;
		padding: 1rem;
		overflow: hidden;
	}
	.log-section h3,
	.npc-interaction-section h3 {
		margin: 0 0 0.8rem 0;
		font-size: 0.95rem;
		color: #9ca3af;
		border-bottom: 1px solid #1f2937;
		padding-bottom: 0.5rem;
	}

	.log-box,
	.npc-chat-container {
		flex: 1;
		background: #030712;
		border: 1px solid #1f2937;
		border-radius: 6px;
		padding: 0.8rem;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		font-size: 0.875rem;
	}

	.log-item {
		font-family: monospace;
		line-height: 1.4;
		color: #d1d5db;
	}
	.log-item .time {
		color: #60a5fa;
		margin-right: 0.4rem;
	}
	.log-item .kind {
		color: #a78bfa;
		margin-right: 0.4rem;
	}
	.empty-log {
		text-align: center;
		color: #4b5563;
		margin: auto;
		font-size: 0.85rem;
	}

	.command-form,
	.npc-form {
		display: flex;
		gap: 0.5rem;
		margin-top: 1rem;
	}
	.command-form input,
	.npc-form input {
		flex: 1;
		background: #030712;
		border: 1px solid #374151;
		border-radius: 6px;
		padding: 0.6rem;
		color: white;
		font-size: 0.9rem;
	}
	.command-form input:focus,
	.npc-form input:focus {
		outline: none;
		border-color: #38bdf8;
	}
	.command-form button,
	.npc-form button {
		background: #2563eb;
		color: white;
		border: none;
		padding: 0 1rem;
		border-radius: 6px;
		font-weight: 600;
		cursor: pointer;
		font-size: 0.85rem;
	}
	.command-form button:hover,
	.npc-form button:hover {
		background: #1d4ed8;
	}
	.command-form button:disabled,
	.npc-form button:disabled {
		background: #4b5563;
		cursor: not-allowed;
	}

	.chat-bubble {
		padding: 0.5rem 0.7rem;
		border-radius: 6px;
		font-size: 0.85rem;
		line-height: 1.4;
		max-width: 85%;
	}
	.chat-bubble.me {
		background: #1e3a8a;
		align-self: flex-end;
		color: #dbeafe;
	}
	.chat-bubble.npc {
		background: #374151;
		align-self: flex-start;
		color: #f3f4f6;
	}
</style>
