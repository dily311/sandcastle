<script lang="ts">
	import { onMount } from "svelte";
	import { ChevronLeft, MapPin, BookOpen, MoreHorizontal, ArrowRight, Clock3 } from "@lucide/svelte";
	import type { GameCommand, StageLine } from "$/lib/type/game";
	import { handleGoBack } from "$/app/routes";
	import { gameStore } from "$/lib/game/gameStore.svelte";
	import Backlog from "./Backlog.svelte";
	import StageView from "./StageView.svelte";

	type Props = { gameId: string; sessionId: string };
	let { gameId, sessionId }: Props = $props();

	let isLoading = $state(false);
	let errorMessage = $state("");
	let showBacklog = $state(false);
	let showTravel = $state(false);
	let scriptLines = $state<StageLine[]>([]);
	let lineIndex = $state(0);

	const currentLine = $derived(scriptLines[lineIndex] ?? null);
	const choices = $derived(gameStore.choices ?? []);
	const destinations = $derived(gameStore.locations ?? []);
	const currentCharacter = $derived(
		currentLine?.speaker
			? gameStore.view?.characters.find((character) => character.id === currentLine.speaker)
			: null,
	);
	const currentScene = $derived(gameStore.currentScene);
	const title = $derived(gameStore.view?.game.title ?? "게임");
	function isNarration(line: StageLine | null) {
		return !line || line.kind === 0 || String(line.kind).toLowerCase() === "narration";
	}

	function acceptResponseLines() {
		const events = gameStore.renderState?.events ?? [];
		const latest = [...events].reverse().find((event) => event.type === "sceneScript" || event.type === "stageLines");
		if (!latest) return;
		if (latest.type === "sceneScript" || latest.type === "stageLines") {
			scriptLines = latest.lines;
			lineIndex = 0;
		}
	}

	async function initSession() {
		try {
			isLoading = true;
			errorMessage = "";
			if (!gameStore.view) await gameStore.loadGame(gameId);
			await gameStore.continueGame(sessionId);
			acceptResponseLines();
		} catch (error) {
			console.error("Failed to initialize game session:", error);
			errorMessage = "게임 세션을 불러오지 못했습니다.";
		} finally {
			isLoading = false;
		}
	}

	async function dispatch(command: GameCommand) {
		if (isLoading) return;
		try {
			isLoading = true;
			errorMessage = "";
			await gameStore.dispatchCommand(command);
			acceptResponseLines();
			showTravel = false;
		} catch (error) {
			console.error("Game command failed:", error);
			errorMessage = "요청을 처리하지 못했습니다. 다시 시도해 주세요.";
		} finally {
			isLoading = false;
		}
	}

	function advanceLine() {
		if (lineIndex < scriptLines.length - 1) lineIndex += 1;
	}

	function formatTime(seconds = 0) {
		const totalMinutes = Math.floor(seconds / 60);
		const hours = Math.floor(totalMinutes / 60) % 24;
		const minutes = totalMinutes % 60;
		return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
	}

	onMount(initSession);
</script>

<main class="vn-player">
	<header class="topbar">
		<button class="quiet-button" onclick={handleGoBack} aria-label="뒤로 가기"><ChevronLeft size={20} /></button>
		<div class="game-heading">
			<strong>{title}</strong>
			<span>{gameStore.currentLocation?.name ?? "장소 불러오는 중"}</span>
		</div>
		<div class="top-actions">
			<div class="time-chip"><Clock3 size={15} /> {formatTime(gameStore.gameTime)}</div>
			<button class="quiet-button" onclick={() => (showBacklog = true)} aria-label="대화 기록"><BookOpen size={18} /></button>
			<button class="quiet-button" onclick={() => (showTravel = !showTravel)} aria-label="이동"><MapPin size={18} /></button>
		</div>
	</header>

	<section class="stage-area" aria-label="게임 장면">
		<StageView />
		<div class="scene-label">
			<span class="scene-dot"></span>
			{currentScene?.name ?? "현재 장면"}
		</div>

		{#if showTravel}
			<aside class="travel-panel" aria-label="이동할 장소">
				<div class="panel-heading"><strong>어디로 갈까요?</strong><button class="quiet-button" onclick={() => (showTravel = false)} aria-label="닫기">×</button></div>
				{#if destinations.length}
					{#each destinations as destination (destination.toId)}
						<button class="destination" disabled={isLoading} onclick={() => dispatch({ type: "moveTo", locationId: destination.toId })}>
							<span><strong>{destination.location?.name}</strong><small>{Math.ceil(destination.travelSeconds / 60)}분 이동</small></span><ArrowRight size={17} />
						</button>
					{/each}
				{:else}<p class="muted">지금 이동할 수 있는 장소가 없습니다.</p>{/if}
			</aside>
		{/if}

		{#if errorMessage}<div class="error-toast" role="alert">{errorMessage}</div>{/if}

		{#if currentLine || currentScene}
			<section class="dialogue-panel" aria-live="polite">
				{#if !isNarration(currentLine) && currentCharacter}
					<div class="speaker">{currentCharacter.name}</div>
				{/if}
				<p class:narration={isNarration(currentLine)}>
					{#if currentLine}{currentLine.text}{:else}{currentScene?.description ?? "장면을 시작합니다."}{/if}
				</p>
				{#if choices.length}
					<div class="choice-list">
						{#each choices as choice (choice.id)}
							<button class="choice-button" disabled={isLoading} onclick={() => dispatch({ type: "choose", choiceId: choice.id })}>{choice.text}</button>
						{/each}
					</div>
				{:else if scriptLines.length > 1 && lineIndex < scriptLines.length - 1}
					<button class="next-button" onclick={advanceLine} disabled={isLoading}>다음 <ArrowRight size={16} /></button>
				{:else if isLoading}<div class="loading-label">장면을 불러오는 중…</div>{/if}
			</section>
		{/if}
		{#if isLoading}<div class="busy-indicator">처리 중…</div>{/if}
	</section>

	<footer class="bottom-bar">
		<span><MapPin size={15} /> {gameStore.currentLocation?.name ?? "장소 없음"}</span>
		<span>{gameStore.currentWeather?.name ?? ""}</span>
		<button class="more-button" onclick={() => (showTravel = !showTravel)}><MoreHorizontal size={18} /> 이동 / 행동</button>
	</footer>
</main>

{#if showBacklog}<Backlog {sessionId} onClose={() => (showBacklog = false)} />{/if}

<style>
	.vn-player { height: 100dvh; min-height: 480px; display: flex; flex-direction: column; overflow: hidden; color: #f8f5f1; background: #11100f; }
	.topbar { height: 62px; flex: 0 0 62px; display: flex; align-items: center; gap: 14px; padding: 0 22px; background: rgba(20,18,17,.94); border-bottom: 1px solid rgba(255,255,255,.08); z-index: 2; }
	.quiet-button { display: inline-grid; place-items: center; width: 38px; height: 38px; border: 0; border-radius: 50%; color: #f2ece6; background: transparent; cursor: pointer; }
	.quiet-button:hover { background: rgba(255,255,255,.1); }
	.game-heading { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
	.game-heading strong { font-size: 14px; font-weight: 650; }
	.game-heading span { color: #b4a9a0; font-size: 12px; }
	.top-actions { display: flex; align-items: center; gap: 7px; margin-left: auto; }
	.time-chip { display: flex; align-items: center; gap: 7px; margin-right: 8px; color: #e6d8ca; font-size: 13px; }
	.stage-area { position: relative; flex: 1; min-height: 0; overflow: hidden; background: #211f22; }
	.scene-label { position: absolute; top: 18px; left: 22px; display: flex; align-items: center; gap: 8px; padding: 8px 12px; border: 1px solid rgba(255,255,255,.16); border-radius: 99px; background: rgba(19,17,17,.55); backdrop-filter: blur(10px); color: #f7eee6; font-size: 12px; }
	.scene-dot { width: 6px; height: 6px; border-radius: 50%; background: #e3ad85; }
	.dialogue-panel { position: absolute; left: 50%; bottom: 26px; transform: translateX(-50%); width: min(880px, calc(100% - 40px)); padding: 20px 24px 18px; border: 1px solid rgba(255,255,255,.2); border-radius: 16px; background: linear-gradient(145deg, rgba(23,20,21,.92), rgba(14,14,17,.91)); box-shadow: 0 18px 60px rgba(0,0,0,.35); backdrop-filter: blur(14px); }
	.speaker { margin-bottom: 9px; color: #f1b78f; font-size: 14px; font-weight: 700; }
	.dialogue-panel p { margin: 0; min-height: 48px; color: #fffaf5; font-size: 17px; line-height: 1.75; white-space: pre-wrap; }
	.dialogue-panel p.narration { color: #e4ddd6; font-size: 15px; }
	.choice-list { display: grid; gap: 9px; margin-top: 17px; }
	.choice-button { width: 100%; padding: 12px 15px; border: 1px solid rgba(236,190,158,.43); border-radius: 10px; text-align: left; color: #fffaf5; background: rgba(205,143,104,.14); cursor: pointer; transition: background .15s, transform .15s; }
	.choice-button:hover:not(:disabled) { background: rgba(205,143,104,.29); transform: translateY(-1px); }
	.choice-button:disabled, .destination:disabled { opacity: .55; cursor: wait; }
	.next-button { display: flex; align-items: center; gap: 8px; margin: 12px 0 0 auto; padding: 8px 12px; border: 0; border-radius: 8px; color: #f5c7a6; background: rgba(255,255,255,.08); cursor: pointer; }
	.loading-label, .muted { color: #b9ada4; font-size: 13px; }
	.bottom-bar { display: flex; align-items: center; gap: 20px; height: 44px; flex: 0 0 44px; padding: 0 22px; background: #181615; border-top: 1px solid rgba(255,255,255,.07); color: #bdb2a9; font-size: 12px; }
	.bottom-bar span { display: inline-flex; align-items: center; gap: 6px; }
	.more-button { display: inline-flex; align-items: center; gap: 5px; margin-left: auto; padding: 6px 10px; border: 0; border-radius: 7px; color: #e6d8ca; background: rgba(255,255,255,.08); cursor: pointer; }
	.travel-panel { position: absolute; z-index: 4; top: 16px; right: 18px; width: min(320px, calc(100% - 36px)); padding: 14px; border: 1px solid rgba(255,255,255,.14); border-radius: 14px; background: rgba(25,22,22,.96); box-shadow: 0 14px 45px #0008; backdrop-filter: blur(14px); }
	.panel-heading { display: flex; align-items: center; justify-content: space-between; padding-left: 7px; margin-bottom: 8px; font-size: 14px; }
	.destination { display: flex; align-items: center; justify-content: space-between; width: 100%; padding: 11px 9px; border: 0; border-radius: 9px; color: #f7eee6; background: transparent; text-align: left; cursor: pointer; }
	.destination:hover:not(:disabled) { background: rgba(255,255,255,.09); }
	.destination span { display: flex; flex-direction: column; gap: 3px; }
	.destination small { color: #aa9d94; }
	.error-toast { position: absolute; z-index: 5; top: 18px; left: 50%; transform: translateX(-50%); padding: 10px 15px; border: 1px solid #e99b8b66; border-radius: 9px; color: #ffe0d9; background: #492722ee; font-size: 13px; }
	.busy-indicator { position: absolute; top: 18px; right: 22px; color: #f4d1b8; font-size: 12px; }
	@media (max-width: 620px) { .topbar { padding: 0 10px; gap: 7px; } .top-actions { gap: 0; } .time-chip { margin-right: 0; } .scene-label { left: 12px; top: 12px; } .dialogue-panel { bottom: 12px; width: calc(100% - 20px); padding: 16px; } .dialogue-panel p { font-size: 15px; } .bottom-bar { padding: 0 12px; gap: 12px; } }
</style>
