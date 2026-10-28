<script lang="ts">
	import PixiMapCanvas from "./PixiMapCanvas.svelte"; // 캔버스 컴포넌트 임포트
	import { handleGoBack } from "$/app/routes";

	let { sessionId }: { sessionId: string } = $props();

	interface RenderState {
		locationId: string | null;
		mapId: string;
		zoneId: string | null;
		gameTime: number;
		weatherID: string;
		timePeriodId: string;
		currentSceneId: string | null;
		activeNpcId: string | null;
		flags: {
			hp?: number;
			maxHp?: number;
			mp?: number;
			maxMp?: number;
			gold?: number;
			inventory?: string[];
			[key: string]: any;
		};
	}

	let renderState = $state<RenderState>({
		locationId: "starting_forest",
		mapId: "whispering_woods",
		zoneId: "zone_1",
		gameTime: 480,
		weatherID: "맑음",
		timePeriodId: "아침",
		currentSceneId: "scene_intro",
		activeNpcId: "village_elder",
		flags: {
			hp: 85,
			maxHp: 100,
			mp: 40,
			maxMp: 50,
			gold: 250,
			inventory: ["녹슨 검", "회복 물약 x2", "낡은 지도"],
		},
	});

	let uiMode = $state<"story" | "dialogue">("story");
	let gameLogs = $state<string[]>([
		"안개 자욱한 속삭임의 숲에 도착했습니다. 찬 공기가 코끝을 스칩니다.",
		"근처에 마을 촌장이 서 있습니다.",
	]);

	let playerInput = $state("");
	let npcInput = $state("");
	let npcDialogues = $state<{ sender: string; text: string }[]>([]);
	let isLoading = $state(false);

	function formatGameTime(minutes: number): string {
		const hours = Math.floor(minutes / 60) % 24;
		const mins = minutes % 60;
		return `${String(hours).padStart(2, "0")}:${String(mins).padStart(2, "0")}`;
	}

	async function handleActionSubmit(e: SubmitEvent) {
		e.preventDefault();
		if (!playerInput.trim() || isLoading) return;

		const cmd = playerInput;
		playerInput = "";
		gameLogs = [`> ${cmd}`, ...gameLogs];

		try {
			isLoading = true;
			gameLogs = [`[이벤트] "${cmd}" 행동을 실행했습니다.`, ...gameLogs];
		} catch (err) {
			console.error(err);
		} finally {
			isLoading = false;
		}
	}

	function handleStartDialogue() {
		if (!renderState.activeNpcId) return;
		uiMode = "dialogue";
		if (npcDialogues.length === 0) {
			npcDialogues = [{ sender: "NPC", text: `안녕하세요, 여행자여. 어떤 일로 나를 찾으셨소?` }];
		}
	}

	function handleEndDialogue() {
		uiMode = "story";
	}

	async function handleNpcTalk(e: SubmitEvent) {
		e.preventDefault();
		if (!npcInput.trim() || isLoading) return;

		const text = npcInput;
		npcInput = "";
		npcDialogues = [...npcDialogues, { sender: "플레이어", text }];

		try {
			isLoading = true;
			setTimeout(() => {
				npcDialogues = [...npcDialogues, { sender: "NPC", text: "조심하게나, 그곳은 매우 위험한 지역이라네." }];
				isLoading = false;
			}, 400);
		} catch (err) {
			console.error(err);
			isLoading = false;
		}
	}
</script>

<div class="fantasy-play-layout">
	<!-- 상단 상태바 -->
	<header class="fantasy-status-bar">
		<div class="brand">⚔️ FANTASY RPG ENGINE</div>

		<div class="status-gauges">
			<div class="gauge-group">
				<span class="label">HP</span>
				<div class="bar-bg">
					<div
						class="bar-fill hp"
						style="width: {(renderState.flags.hp! / renderState.flags.maxHp!) * 100}%"
					></div>
				</div>
				<span class="val">{renderState.flags.hp}/{renderState.flags.maxHp}</span>
			</div>

			<div class="gauge-group">
				<span class="label">MP</span>
				<div class="bar-bg">
					<div
						class="bar-fill mp"
						style="width: {(renderState.flags.mp! / renderState.flags.maxMp!) * 100}%"
					></div>
				</div>
				<span class="val">{renderState.flags.mp}/{renderState.flags.maxMp}</span>
			</div>

			<div class="stat-badge gold">🪙 {renderState.flags.gold} G</div>
		</div>

		<div class="world-meta">
			<span>⏰ {formatGameTime(renderState.gameTime)} ({renderState.timePeriodId})</span>
			<span>⛅ {renderState.weatherID}</span>
			<button class="exit-btn" onclick={handleGoBack}>나가기</button>
		</div>
	</header>

	<!-- 메인 게임 영역 -->
	<main class="fantasy-grid">
		<!-- 메인 텍스트 창 (스토리 로그 또는 NPC 대화창) -->
		<section class="panel main-content-panel">
			{#if uiMode === "story"}
				<div class="panel-header">
					<h3>📜 모험의 기록</h3>
				</div>
				<div class="scroll-box log-box">
					{#each gameLogs as log}
						<div class="log-item">{log}</div>
					{/each}
				</div>

				<form class="action-form" onsubmit={handleActionSubmit}>
					<input
						type="text"
						bind:value={playerInput}
						placeholder="행동을 입력하세요 (예: 동굴로 이동, 주변 조사)..."
						disabled={isLoading}
					/>
					<button type="submit" disabled={isLoading}>행동</button>
				</form>
			{:else}
				<div class="panel-header dialogue-header">
					<h3>💬 대화: <span class="npc-name">{renderState.activeNpcId}</span></h3>
					<button class="close-dialogue-btn" onclick={handleEndDialogue}>대화 종료 ✖</button>
				</div>
				<div class="scroll-list dialogue-box">
					{#each npcDialogues as chat}
						<div class="chat-bubble {chat.sender === '플레이어' ? 'me' : 'npc'}">
							<strong>{chat.sender}:</strong>
							{chat.text}
						</div>
					{/each}
				</div>

				<form class="action-form dialogue-form" onsubmit={handleNpcTalk}>
					<input
						type="text"
						bind:value={npcInput}
						placeholder="NPC에게 건넬 말을 입력하세요..."
						disabled={isLoading}
					/>
					<button type="submit" class="talk-btn" disabled={isLoading}>말하기</button>
				</form>
			{/if}
		</section>

		<!-- 우측 사이드바 (Pixi 캔버스 및 인벤토리 포함) -->
		<section class="panel sidebar-panel">
			<!-- Pixi.js 2D 맵/그래픽 캔버스 영역 -->
			<div class="canvas-box">
				<PixiMapCanvas
					mapId={renderState.mapId}
					locationId={renderState.locationId}
					activeNpcId={renderState.activeNpcId}
				/>
			</div>

			<!-- NPC 상호작용 액션 박스 -->
			<div class="npc-action-box">
				<h4>👥 현재 지역의 인물</h4>
				{#if renderState.activeNpcId}
					<div class="npc-card">
						<div class="npc-info">
							<span class="icon">👤</span>
							<span class="name">{renderState.activeNpcId}</span>
						</div>
						{#if uiMode === "story"}
							<button class="talk-trigger-btn" onclick={handleStartDialogue}>💬 대화하기</button>
						{:else}
							<span class="in-talk-badge">대화 진행 중...</span>
						{/if}
					</div>
				{:else}
					<p class="no-npc-text">주변에 상호작용할 인물이 없습니다.</p>
				{/if}
			</div>

			<!-- 인벤토리 소지품 -->
			<div class="inventory-box">
				<h4>🎒 인벤토리 소지품</h4>
				<ul class="item-list">
					{#each renderState.flags.inventory || [] as item}
						<li>✨ {item}</li>
					{:else}
						<li class="empty">가방이 비어 있습니다.</li>
					{/each}
				</ul>
			</div>
		</section>
	</main>
</div>

<style>
	.fantasy-play-layout {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background: #0c0a09;
		color: #d1d5db;
		font-family: "Georgia", serif;
		background-image: radial-gradient(circle at center, #1c1917 0%, #0c0a09 100%);
	}

	.fantasy-status-bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.8rem 1.5rem;
		background: #17120e;
		border-bottom: 2px solid #b45309;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
	}

	.brand {
		font-weight: bold;
		color: #f59e0b;
		font-size: 1.1rem;
		letter-spacing: 1px;
	}
	.status-gauges {
		display: flex;
		gap: 1.5rem;
		align-items: center;
	}
	.gauge-group {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.85rem;
		font-family: monospace;
	}
	.gauge-group .label {
		font-weight: bold;
		color: #9ca3af;
	}

	.bar-bg {
		width: 100px;
		height: 10px;
		background: #292524;
		border-radius: 5px;
		overflow: hidden;
		border: 1px solid #44403c;
	}
	.bar-fill {
		height: 100%;
		transition: width 0.3s ease;
	}
	.bar-fill.hp {
		background: #dc2626;
	}
	.bar-fill.mp {
		background: #2563eb;
	}
	.val {
		font-size: 0.75rem;
		color: #d1d5db;
		width: 45px;
	}

	.stat-badge {
		background: #292524;
		padding: 0.3rem 0.8rem;
		border-radius: 6px;
		border: 1px solid #b45309;
		color: #fbbf24;
		font-weight: bold;
		font-size: 0.85rem;
	}
	.world-meta {
		display: flex;
		align-items: center;
		gap: 1rem;
		font-size: 0.85rem;
		color: #a855f7;
	}
	.exit-btn {
		background: #78350f;
		color: #fef3c7;
		border: 1px solid #b45309;
		padding: 0.3rem 0.8rem;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.8rem;
	}
	.exit-btn:hover {
		background: #92400e;
	}

	.fantasy-grid {
		display: grid;
		grid-template-columns: 1fr 340px;
		gap: 1.2rem;
		flex: 1;
		padding: 1.2rem;
		overflow: hidden;
	}

	.panel {
		background: #17120e;
		border: 1px solid #44403c;
		border-radius: 10px;
		display: flex;
		flex-direction: column;
		padding: 1rem;
		overflow: hidden;
		box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.6);
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		border-bottom: 1px solid #292524;
		padding-bottom: 0.5rem;
		margin-bottom: 0.8rem;
	}
	.panel-header h3 {
		margin: 0;
		font-size: 0.95rem;
		color: #fbbf24;
	}
	.panel h4 {
		margin: 0 0 0.5rem 0;
		font-size: 0.85rem;
		color: #f59e0b;
	}
	.npc-name {
		color: #38bdf8;
		font-weight: bold;
	}

	.close-dialogue-btn {
		background: #991b1b;
		color: #fef2f2;
		border: none;
		padding: 0.3rem 0.6rem;
		border-radius: 4px;
		font-size: 0.75rem;
		cursor: pointer;
	}
	.close-dialogue-btn:hover {
		background: #dc2626;
	}

	.scroll-box,
	.scroll-list {
		flex: 1;
		background: #0c0a09;
		border: 1px solid #292524;
		border-radius: 6px;
		padding: 0.8rem;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}

	.log-item {
		font-size: 0.9rem;
		line-height: 1.5;
		color: #e7e5e4;
	}
	.chat-bubble {
		padding: 0.6rem 0.8rem;
		border-radius: 6px;
		font-size: 0.875rem;
		line-height: 1.4;
		max-width: 80%;
	}
	.chat-bubble.me {
		background: #451a03;
		align-self: flex-end;
		color: #fef3c7;
		border: 1px solid #78350f;
	}
	.chat-bubble.npc {
		background: #1c1917;
		align-self: flex-start;
		color: #f3f4f6;
		border: 1px solid #44403c;
	}

	.action-form {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.8rem;
	}
	.action-form input {
		flex: 1;
		background: #0c0a09;
		border: 1px solid #44403c;
		border-radius: 6px;
		padding: 0.6rem;
		color: #fef3c7;
		font-size: 0.9rem;
	}
	.action-form input:focus {
		outline: none;
		border-color: #f59e0b;
	}
	.action-form button {
		background: #b45309;
		color: white;
		border: none;
		padding: 0 1.2rem;
		border-radius: 6px;
		font-weight: bold;
		cursor: pointer;
		font-size: 0.85rem;
	}
	.action-form button.talk-btn {
		background: #2563eb;
	}
	.action-form button.talk-btn:hover {
		background: #1d4ed8;
	}
	.action-form button:hover {
		background: #d97706;
	}

	.sidebar-panel {
		display: flex;
		flex-direction: column;
		gap: 0.8rem;
	}
	.canvas-box {
		flex-shrink: 0;
	}

	.npc-action-box {
		background: #0c0a09;
		border: 1px solid #292524;
		padding: 0.8rem;
		border-radius: 6px;
	}
	.npc-card {
		display: flex;
		justify-content: space-between;
		align-items: center;
		background: #1c1917;
		padding: 0.6rem 0.8rem;
		border-radius: 6px;
		border: 1px solid #44403c;
		margin-top: 0.4rem;
	}
	.npc-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.85rem;
		color: #f3f4f6;
		font-weight: bold;
	}

	.talk-trigger-btn {
		background: #2563eb;
		color: white;
		border: none;
		padding: 0.4rem 0.8rem;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: bold;
		cursor: pointer;
	}
	.talk-trigger-btn:hover {
		background: #1d4ed8;
	}
	.in-talk-badge {
		font-size: 0.75rem;
		color: #38bdf8;
		font-style: italic;
	}
	.no-npc-text {
		margin: 0.4rem 0 0 0;
		font-size: 0.8rem;
		color: #78716c;
	}

	.inventory-box {
		flex: 1;
		background: #0c0a09;
		border: 1px solid #292524;
		padding: 0.8rem;
		border-radius: 6px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.item-list {
		list-style: none;
		padding: 0;
		margin: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		font-size: 0.85rem;
		color: #d6d3d1;
	}
	.item-list .empty {
		color: #57534e;
		font-style: italic;
	}
</style>
