<script lang="ts">
	import { onMount, tick } from "svelte";
	import type { AssetRef, GameCommand, GameJournal, StageLine } from "$/lib/type/game";
	import { gameStore } from "$/lib/game/gameStore.svelte";
	import { gameEngineCommands } from "$/ipc/commands";
	import { handleGoBack } from "$/app/routes";
	import Backlog from "./Backlog.svelte";
	import StageView from "./StageView.svelte";
	import PlayHeader from "./PlayHeader.svelte";
	import PlayPanels from "./PlayPanels.svelte";
	import DialoguePanel from "./DialoguePanel.svelte";
	import ItemGainNotices from "./ItemGainNotices.svelte";
	import PlayBottomBar from "./PlayBottomBar.svelte";
	import { getItemAddedEvents, getStageLines, normalizeCueType, type ItemGainNotice } from "./playUtils";

	type Props = { gameId: string; sessionId: string };
	let { gameId, sessionId }: Props = $props();
	type DisplayStanding = { emotion: string; asset: AssetRef };
	type CurrentBgm = { mood: string; asset: AssetRef };

	let isLoading = $state(false);
	let showBacklog = $state(false);
	let showMap = $state(false);
	let showActions = $state(false);
	let showInventory = $state(false);
	let showWaitMenu = $state(false);
	let scriptLines = $state<StageLine[]>([]);
	let lineIndex = $state(0);
	let standingOverrides = $state<Record<string, DisplayStanding | null>>({});
	let standingBaseline: Record<string, DisplayStanding | null> = {};
	let currentBgm = $state<CurrentBgm | null>(null);
	let scriptStartBgm: CurrentBgm | null = null;
	let bgmBlocked = $state(false);
	let isBgmMuted = $state(false);
	let bgmAudio: HTMLAudioElement;
	let itemGainNotices = $state<ItemGainNotice[]>([]);
	let nextItemGainNoticeId = 0;

	const currentLine = $derived(scriptLines[lineIndex] ?? null);
	const choices = $derived(gameStore.choices ?? []);
	const isReadingScript = $derived(scriptLines.length > 0 && lineIndex < scriptLines.length - 1);
	const canWait = $derived(!gameStore.state?.currentSceneId);
	const currentCharacter = $derived(
		currentLine?.speaker ? gameStore.view?.characters.find((character) => character.id === currentLine.speaker) : null,
	);
	const speakerName = $derived(currentLine?.speaker === "player" ? "나" : (currentCharacter?.name ?? null));
	const focusedCharacterId = $derived(currentCharacter?.id ?? null);
	const currentScene = $derived(gameStore.currentScene);
	const title = $derived(gameStore.view?.game.title ?? "게임");

	function setBgm(asset: AssetRef | null, mood = "") {
		if (!asset) {
			currentBgm = null;
			bgmBlocked = false;
			if (bgmAudio) bgmAudio.pause();
			return;
		}
		if (currentBgm?.asset.id === asset.id) {
			currentBgm = { mood, asset };
			return;
		}
		currentBgm = { mood, asset };
		bgmBlocked = false;
		void tick().then(async () => {
			if (!bgmAudio || currentBgm?.asset.id !== asset.id) return;
			const volume = Number(asset.settings?.volume);
			bgmAudio.volume = Number.isFinite(volume) ? Math.max(0, Math.min(1, volume)) : 0.5;
			bgmAudio.muted = isBgmMuted;
			bgmAudio.loop = asset.settings?.loop !== false;
			bgmAudio.currentTime = 0;
			try {
				await bgmAudio.play();
			} catch {
				bgmBlocked = true;
			}
		});
	}

	function resumeBgm() {
		if (!bgmAudio) return;
		void bgmAudio.play().then(() => (bgmBlocked = false)).catch(() => (bgmBlocked = true));
	}

	function toggleBgmMute() {
		isBgmMuted = !isBgmMuted;
		if (bgmAudio) bgmAudio.muted = isBgmMuted;
	}

	$effect(() => {
		if (bgmAudio) bgmAudio.muted = isBgmMuted;
	});

	function syncBgmFromStage() {
		const bgm = gameStore.stage?.bgm as unknown as { mood?: string; asset?: AssetRef | null } | undefined;
		setBgm(bgm?.asset ?? null, bgm?.mood ?? "");
	}

	function applyLineCues(line: StageLine | undefined): CurrentBgm | null | undefined {
		if (!line) return undefined;
		let nextBgm: { asset: AssetRef | null; mood: string } | undefined;
		for (const rawCue of line.cues ?? []) {
			const cue = rawCue as unknown as {
				type?: string;
				characterId?: string;
				emotion?: string;
				asset?: AssetRef | null;
				mood?: string;
			};
			switch (normalizeCueType(cue.type)) {
				case "standing":
					if (cue.characterId && cue.asset) {
						standingOverrides = { ...standingOverrides, [cue.characterId]: { emotion: cue.emotion ?? "normal", asset: cue.asset } };
					}
					break;
				case "standing_off":
					if (cue.characterId) standingOverrides = { ...standingOverrides, [cue.characterId]: null };
					break;
				case "bgm":
					nextBgm = { asset: cue.asset ?? null, mood: cue.mood ?? "" };
					break;
			}
		}
		return nextBgm ? (nextBgm.asset ? { asset: nextBgm.asset, mood: nextBgm.mood } : null) : undefined;
	}

	function rebuildLinePresentation(index: number) {
		standingOverrides = { ...standingBaseline };
		let targetBgm = scriptStartBgm;
		for (let line = 0; line <= index; line += 1) {
			const cueBgm = applyLineCues(scriptLines[line]);
			if (cueBgm !== undefined) targetBgm = cueBgm;
		}
		setBgm(targetBgm?.asset ?? null, targetBgm?.mood ?? "");
	}

	function prepareLinePresentation(lines: StageLine[]) {
		const affectedCharacters = new Set<string>();
		let hasBgmCue = false;
		for (const line of lines) {
			for (const rawCue of line.cues ?? []) {
				const cue = rawCue as unknown as { type?: string; characterId?: string };
				if (normalizeCueType(cue.type) === "bgm") hasBgmCue = true;
				if (["standing", "standing_off"].includes(normalizeCueType(cue.type)) && cue.characterId) affectedCharacters.add(cue.characterId);
			}
		}
		const baseline: Record<string, DisplayStanding | null> = { ...(gameStore.stage?.standings ?? {}) };
		for (const characterId of affectedCharacters) {
			const normal = gameStore.view?.assets.characters.find(
				(asset) => asset.characterId === characterId && asset.usage === "standing" && asset.emotion.toLowerCase() === "normal",
			);
			baseline[characterId] = normal ? { emotion: normal.emotion, asset: normal } : null;
		}
		if (!hasBgmCue) syncBgmFromStage();
		standingBaseline = baseline;
		scriptStartBgm = currentBgm ? { ...currentBgm } : null;
		rebuildLinePresentation(0);
	}

	function acceptResponseLines() {
		const lines = getStageLines(gameStore.renderState?.events ?? []);
		if (!lines.length) return false;
		scriptLines = lines;
		lineIndex = 0;
		prepareLinePresentation(lines);
		return true;
	}

	async function restoreCurrentSceneLines() {
		if (scriptLines.length || !gameStore.state?.currentSceneId) return;
		const sessions = await gameEngineCommands.listSessions();
		const session = sessions.find((item) => item.id === sessionId);
		const toSeq = session?.headSeq ?? 1000;
		const history: GameJournal[] = await gameEngineCommands.readJournal({ sessionId, fromSeq: Math.max(0, toSeq - 1000), toSeq });
		const savedScript = [...history]
			.sort((a, b) => a.seq - b.seq)
			.reverse()
			.map((row) => row.payloadJson as { type?: string; sceneId?: string; lines?: unknown })
			.find((payload) => payload.type === "sceneScript" && payload.sceneId === gameStore.state?.currentSceneId && Array.isArray(payload.lines));
		if (Array.isArray(savedScript?.lines) && savedScript.lines.length) {
			scriptLines = savedScript.lines as StageLine[];
			lineIndex = 0;
			prepareLinePresentation(scriptLines);
		}
	}

	async function initSession() {
		try {
			isLoading = true;
			if (!gameStore.view) await gameStore.loadGame(gameId);
			if (gameStore.state?.sessionId !== sessionId) await gameStore.continueGame(sessionId);
			if (!acceptResponseLines()) syncBgmFromStage();
			await restoreCurrentSceneLines();
		} catch (error) {
			console.error("Failed to initialize game session:", error);
		} finally {
			isLoading = false;
		}
	}

	function showItemGains() {
		const addedItems = getItemAddedEvents(gameStore.renderState?.events ?? []);
		for (const event of addedItems) {
			const item = gameStore.view?.items.find((candidate) => candidate.id === event.itemId);
			const id = nextItemGainNoticeId++;
			itemGainNotices = [...itemGainNotices, { id, name: item?.name ?? "아이템", quantity: event.quantity }];
			setTimeout(() => (itemGainNotices = itemGainNotices.filter((notice) => notice.id !== id)), 3600);
		}
	}

	async function dispatch(command: GameCommand) {
		if (isLoading) return;
		const previousLines = scriptLines;
		const previousIndex = lineIndex;
		try {
			isLoading = true;
			scriptLines = [];
			lineIndex = 0;
			await gameStore.dispatchCommand(command);
			showItemGains();
			if (!acceptResponseLines()) {
				standingOverrides = {};
				syncBgmFromStage();
			}
			showActions = false;
			showMap = false;
			showInventory = false;
			showWaitMenu = false;
		} catch (error) {
			console.error("Game command failed:", error);
			scriptLines = previousLines;
			lineIndex = previousIndex;
		} finally {
			isLoading = false;
		}
	}

	function advanceLine() {
		if (lineIndex < scriptLines.length - 1) {
			lineIndex += 1;
			rebuildLinePresentation(lineIndex);
		}
	}

	function previousLine() {
		if (lineIndex > 0) {
			lineIndex -= 1;
			rebuildLinePresentation(lineIndex);
		}
	}

	function handleStageClick() {
		if (!isLoading && !showActions && !showMap && !showInventory && !showBacklog) advanceLine();
	}

	function toggleMap() {
		showMap = !showMap;
		showWaitMenu = false;
		if (showMap) { showActions = false; showInventory = false; }
	}

	function toggleActions() {
		showActions = !showActions;
		showWaitMenu = false;
		if (showActions) { showMap = false; showInventory = false; }
	}

	function toggleInventory() {
		showInventory = !showInventory;
		showWaitMenu = false;
		if (showInventory) { showMap = false; showActions = false; }
	}

	function toggleWaitMenu() {
		showWaitMenu = !showWaitMenu;
		if (showWaitMenu) { showMap = false; showActions = false; showInventory = false; }
	}

	onMount(initSession);
</script>

<main class="vn-player">
	<PlayHeader
		{title}
		locationName={gameStore.currentLocation?.name ?? "장소 불러오는 중"}
		gameTime={gameStore.gameTime}
		canWait
		{isLoading}
		{showWaitMenu}
		{isBgmMuted}
		onBack={handleGoBack}
		onToggleWaitMenu={toggleWaitMenu}
		onWait={(hours) => dispatch({ type: "advanceTime", minutes: hours * 60 })}
		onBacklog={() => (showBacklog = true)}
		onInventory={toggleInventory}
		onToggleMute={toggleBgmMute}
		onMap={toggleMap}
	/>

	<section class="stage-area" aria-labelledby="scene-name">
		<StageView onAdvance={handleStageClick} {standingOverrides} {focusedCharacterId} />
		<div class="scene-label" id="scene-name"><span class="scene-dot"></span>{currentScene?.name ?? "현재 장면"}</div>
		<PlayPanels
			showMap={showMap}
			showActions={showActions}
			showInventory={showInventory}
			{isLoading}
			canMove={canWait}
			onCloseMap={() => (showMap = false)}
			onCloseActions={() => (showActions = false)}
			onCloseInventory={() => (showInventory = false)}
			onDispatch={dispatch}
		/>
		<ItemGainNotices notices={itemGainNotices} />
		<DialoguePanel
			show={Boolean(currentLine || currentScene)}
			line={currentLine}
			{speakerName}
			sceneDescription={currentScene?.description ?? null}
			{choices}
			{lineIndex}
			lineCount={scriptLines.length}
			{isReadingScript}
			{isLoading}
			onStageClick={handleStageClick}
			onPrevious={previousLine}
			onAdvance={advanceLine}
			onChoose={(choiceId) => dispatch({ type: "choose", choiceId })}
		/>
		{#if isLoading}<div class="busy-indicator">처리 중…</div>{/if}
	</section>

	<PlayBottomBar
		locationName={gameStore.currentLocation?.name ?? "장소 없음"}
		weatherName={gameStore.currentWeather?.name ?? ""}
		{bgmBlocked}
		onActions={toggleActions}
		onResumeBgm={resumeBgm}
	/>
	<audio class="bgm-audio" bind:this={bgmAudio} src={currentBgm ? gameStore.getAssetUrl(currentBgm.asset) : undefined} loop={currentBgm?.asset.settings?.loop !== false} preload="auto"></audio>
</main>

{#if showBacklog}<Backlog {sessionId} onClose={() => (showBacklog = false)} />{/if}

<style>
	.vn-player { height: calc(100dvh - var(--titlebar-height)); min-height: 0; display: flex; flex-direction: column; overflow: hidden; color: var(--game-text-main); background: var(--game-player-bg); }
	:global(body.is-frameless) .vn-player { height: 100dvh; }
	.stage-area { position: relative; flex: 1; min-height: 0; overflow: hidden; background: var(--game-stage-bg); isolation: isolate; }
	.scene-label { position: absolute; z-index: 20; top: 18px; left: 22px; display: flex; align-items: center; gap: 8px; padding: 8px 12px; border: 1px solid var(--game-border); border-radius: 99px; color: var(--game-text-main); background: var(--game-scene-label-bg); backdrop-filter: blur(10px); font-size: 12px; }
	.scene-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--game-accent); }
	.bgm-audio { display: none; }
	.busy-indicator { position: absolute; z-index: 150; top: 18px; right: 22px; color: var(--game-accent-strong); font-size: 12px; }
	@media (max-width: 620px) { .scene-label { top: 12px; left: 12px; } }
</style>
