<script lang="ts">
	import { onMount, onDestroy } from "svelte";

	import BacklogPanel from "./BacklogPanel.svelte";

	import type { NpcResponder } from "./npc/npcResponder";
	import { GameStageController, type GameStageDisplay, type LogEntry } from "./stage/GameStageController.svelte";

	interface Props {
		gameId: string;
		startSceneKey: string;
		npcResponder: NpcResponder;
		slotNumber: number;
		showDebug: boolean;
		onExit?: () => void;
	}

	let { gameId, startSceneKey, npcResponder, slotNumber, showDebug = $bindable(), onExit }: Props = $props();

	let canvasHost: HTMLDivElement;
	let iframeEl: HTMLIFrameElement;

	let controller: GameStageController;

	let display = $state<GameStageDisplay | null>(null);
	let debugNote = $state("iframe 로드 대기 중...");
	let saveFeedback = $state<string | null>(null);
	let forkFeedback = $state<string | null>(null);
	let log = $state<LogEntry[]>([]);
	let showBacklog = $state(false);

	async function handleSave() {
		const result = await controller.manualSave();
		saveFeedback = result.ok ? "저장했어요" : "저장할 게 없어요";
		setTimeout(() => (saveFeedback = null), 1500);
	}

	async function handleFork() {
		const result = await controller.forkFromCurrentView();
		forkFeedback = result.ok ? `슬롯 ${result.slotNumber}에 분기했어요` : (result.error ?? "분기할 수 없어요");
		setTimeout(() => (forkFeedback = null), 2000);
	}

	onMount(async () => {
		controller = new GameStageController({
			gameId,
			startSceneKey,
			slotNumber,
			npcResponder,

			canvasHost,
			iframeEl,

			onDisplay: (next) => {
				display = next;
			},

			onDebug: (message) => {
				debugNote = message;
			},

			onLogUpdate: (next) => {
				log = next;
			},
		});

		await controller.start();
	});

	onDestroy(() => {
		controller?.destroy();
	});
</script>

<div class="game-stage">
	<div class="canvas-host" bind:this={canvasHost}></div>

	<iframe
		bind:this={iframeEl}
		src="/game-runtime.html"
		sandbox="allow-scripts"
		title="game runtime"
		class="game-iframe"
	></iframe>

	<div class="top-bar">
		{#if onExit}
			<button class="icon-btn" onclick={onExit} title="타이틀로 나가기">☰ 메뉴</button>
		{/if}
		<button class="icon-btn" onclick={handleSave} title="지금까지 진행 저장">💾 저장</button>
		{#if saveFeedback}
			<span class="save-feedback">{saveFeedback}</span>
		{/if}
		<button class="icon-btn" onclick={() => (showBacklog = true)} title="지난 대사 다시 보기"> 📜 로그 </button>
		<button class="icon-btn" onclick={() => (showDebug = !showDebug)} title="디버그 정보"> 🐛 </button>
	</div>

	{#if display?.kind === "line"}
		<div class="overlay">
			<div class="dialogue-toolbar">
				<button class="tool-btn" disabled={!controller.canGoPrevious()} onclick={() => controller.previous()}>
					◀ 이전
				</button>
				<button class="tool-btn" disabled={!controller.canRegenerate()} onclick={() => controller.regenerate()}>
					🔄 재생성
				</button>
				{#if controller.swipeInfo()}
					{@const info = controller.swipeInfo()}
					<button
						class="tool-btn"
						disabled={!controller.canSwipe(-1)}
						onclick={() => controller.goToSwipe(-1)}
						title="이전 버전 보기"
					>
						◁
					</button>
					<span class="swipe-count">
						{info?.index + 1}/{info?.total}
					</span>
					<button
						class="tool-btn"
						disabled={!controller.canSwipe(1)}
						onclick={() => controller.goToSwipe(1)}
						title="다음 버전 보기"
					>
						▷
					</button>
				{/if}
				<button class="tool-btn" disabled={!controller.canFork()} onclick={handleFork}> 🌿 분기 </button>
				{#if forkFeedback}
					<span class="fork-feedback">{forkFeedback}</span>
				{/if}
				<button class="tool-btn next" onclick={() => controller.next()}> 다음 ▶ </button>
			</div>

			<button class="textbox" onclick={() => controller.next()}>
				{#if display.speaker}
					<div class="speaker">
						{display.speaker}
					</div>
				{/if}

				<p class="text">
					{display.text}
				</p>

				<div class="hint">
					{controller.isScrubbing() ? "지난 대사를 보는 중" : "클릭해서 계속"}
				</div>
			</button>
		</div>
	{:else if display?.kind === "choice"}
		<div class="overlay">
			<div class="choice-box">
				{#each display.choices as choice, i (choice.text + i)}
					<button class="choice" onclick={() => controller.selectChoice(i)}>
						{choice.text}
					</button>
				{/each}
			</div>
		</div>
	{/if}

	{#if showDebug}
		<div class="debug-panel">
			{debugNote}
		</div>
	{/if}

	{#if showBacklog}
		<BacklogPanel {log} onClose={() => (showBacklog = false)} />
	{/if}
</div>

<style>
	.game-stage {
		position: relative;
		width: 960px;
		max-width: 100%;
	}

	.canvas-host {
		width: 100%;
		aspect-ratio: 960 / 540;
		line-height: 0;
	}

	.canvas-host :global(canvas) {
		width: 100%;
		height: 100%;
		display: block;
	}

	.game-iframe {
		position: absolute;
		width: 1px;
		height: 1px;
		opacity: 0;
		pointer-events: none;
		border: 0;
	}

	.overlay {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		padding: 16px;
		box-sizing: border-box;
	}

	.dialogue-toolbar {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 6px;
		flex-wrap: wrap;
	}

	.swipe-count {
		font-size: 11px;
		color: rgba(255, 255, 255, 0.6);
	}

	.tool-btn {
		background: rgba(0, 0, 0, 0.6);
		border: 1px solid rgba(255, 255, 255, 0.2);
		border-radius: 6px;
		padding: 5px 10px;
		color: #f2f2f2;
		cursor: pointer;
		font-size: 12px;
	}

	.tool-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.tool-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.15);
	}

	.tool-btn.next {
		margin-left: auto;
		background: rgba(255, 209, 102, 0.2);
		border-color: #ffd166;
	}

	.fork-feedback {
		font-size: 12px;
		color: #9ae6b4;
	}

	.textbox {
		display: block;
		width: 100%;
		background: rgba(0, 0, 0, 0.75);
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: 8px;
		padding: 16px 20px;
		cursor: pointer;
		color: #f2f2f2;
		text-align: left;
		font: inherit;
	}

	.textbox.ended {
		cursor: default;
	}

	.speaker {
		font-weight: 600;
		color: #ffd166;
		margin-bottom: 4px;
	}

	.text {
		margin: 0;
		line-height: 1.6;
	}

	.hint {
		margin-top: 8px;
		font-size: 12px;
		color: rgba(255, 255, 255, 0.5);
		text-align: right;
	}

	.choice-box {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.choice {
		background: rgba(0, 0, 0, 0.75);
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: 8px;
		padding: 12px 16px;
		color: #f2f2f2;
		text-align: left;
		cursor: pointer;
		font-size: 14px;
		font-family: inherit;
	}

	.choice:hover {
		background: rgba(255, 209, 102, 0.15);
		border-color: #ffd166;
	}

	.top-bar {
		position: absolute;
		top: 8px;
		left: 8px;
		display: flex;
		gap: 6px;
		z-index: 2;
	}

	.icon-btn {
		background: rgba(0, 0, 0, 0.55);
		border: 1px solid rgba(255, 255, 255, 0.2);
		border-radius: 6px;
		padding: 4px 10px;
		color: #f2f2f2;
		cursor: pointer;
		font-size: 12px;
	}

	.icon-btn:hover {
		background: rgba(0, 0, 0, 0.75);
	}

	.save-feedback {
		background: rgba(47, 133, 90, 0.85);
		border-radius: 6px;
		padding: 4px 10px;
		font-size: 12px;
		color: #fff;
		align-self: center;
	}

	.debug-panel {
		position: absolute;
		top: 8px;
		right: 8px;
		background: rgba(0, 0, 0, 0.6);
		color: #9ae6b4;
		font-family: monospace;
		font-size: 11px;
		padding: 6px 10px;
		border-radius: 6px;
	}
</style>
