<script lang="ts">
	import { onMount, tick } from "svelte";
	import { ChevronLeft, MapPin, Map as MapIcon, BookOpen, MoreHorizontal, ArrowRight, Clock3, X, Package, Volume2, VolumeX } from "@lucide/svelte";
	import type { AssetRef, GameCommand, GameJournal, StageLine } from "$/lib/type/game";
	import { handleGoBack } from "$/app/routes";
	import { gameStore } from "$/lib/game/gameStore.svelte";
	import { gameEngineCommands } from "$/ipc/commands";
	import Backlog from "./Backlog.svelte";
	import StageView from "./StageView.svelte";
	import Header from "$/components/Header.svelte";

	type Props = { gameId: string; sessionId: string };
	let { gameId, sessionId }: Props = $props();

	let isLoading = $state(false);
	let errorMessage = $state("");
	let showBacklog = $state(false);
	let showMap = $state(false);
	let showActions = $state(false);
	let showInventory = $state(false);
	let scriptLines = $state<StageLine[]>([]);
	let lineIndex = $state(0);
	type DisplayStanding = { emotion: string; asset: AssetRef };
	type CurrentBgm = { mood: string; asset: AssetRef };
	let standingOverrides = $state<Record<string, DisplayStanding | null>>({});
	let standingBaseline: Record<string, DisplayStanding | null> = {};
	let currentBgm = $state<CurrentBgm | null>(null);
	let scriptStartBgm: CurrentBgm | null = null;
	let bgmBlocked = $state(false);
	let isBgmMuted = $state(false);
	let bgmAudio: HTMLAudioElement;

	const currentLine = $derived(scriptLines[lineIndex] ?? null);
	const choices = $derived(gameStore.choices ?? []);
	const destinations = $derived(gameStore.locations ?? []);
	const inventoryEntries = $derived(
		Object.entries((gameStore.state?.inventory ?? {}) as Record<string, number>)
			.filter(([, quantity]) => typeof quantity === "number" && quantity > 0)
			.map(([itemId, quantity]) => ({
				itemId,
				quantity,
				item: gameStore.view?.items.find((item) => item.id === itemId),
			})),
	);
	const mapZones = $derived(
		(gameStore.view?.zones ?? [])
			.filter((zone) => zone.mapId === gameStore.state?.mapId)
			.map((zone) => ({
				...zone,
				locations: (gameStore.view?.locations ?? []).filter((location) => location.zoneId === zone.id),
			})),
	);
	const nearbyCharacters = $derived(
		(gameStore.view?.characters ?? []).filter((character) => gameStore.state?.presentNpcs.includes(character.id)),
	);
	const localInteractables = $derived(
		(gameStore.view?.interactables ?? []).filter((item) => item.locationId === gameStore.state?.locationId),
	);
	const isReadingScript = $derived(scriptLines.length > 0 && lineIndex < scriptLines.length - 1);
	const canMove = $derived(!gameStore.state?.currentSceneId);
	const currentCharacter = $derived(
		currentLine?.speaker
			? gameStore.view?.characters.find((character) => character.id === currentLine.speaker)
			: null,
	);
	const speakerName = $derived(currentLine?.speaker === "player" ? "나" : (currentCharacter?.name ?? null));
	const focusedCharacterId = $derived(currentCharacter?.id ?? null);
	const currentScene = $derived(gameStore.currentScene);
	const title = $derived(gameStore.view?.game.title ?? "게임");
	function isNarration(line: StageLine | null) {
		return !line || line.kind === 0 || String(line.kind).toLowerCase() === "narration";
	}

	function cueType(type: string | undefined) {
		return (type ?? "").replace(/([a-z])([A-Z])/g, "$1_$2").toLowerCase();
	}

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
			switch (cueType(cue.type)) {
				case "standing":
					if (cue.characterId && cue.asset) {
						standingOverrides = {
							...standingOverrides,
							[cue.characterId]: { emotion: cue.emotion ?? "normal", asset: cue.asset },
						};
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
				if (cueType(cue.type) === "bgm") hasBgmCue = true;
				if (["standing", "standing_off"].includes(cueType(cue.type)) && cue.characterId) {
					affectedCharacters.add(cue.characterId);
				}
			}
		}
		const baseline: Record<string, DisplayStanding | null> = {
			...(gameStore.stage?.standings ?? {}),
		};
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
		const events = gameStore.renderState?.events ?? [];
		const sceneScript = [...events].reverse().find((event) => event.type === "sceneScript");
		const lines =
			sceneScript?.type === "sceneScript"
				? sceneScript.lines
				: events.flatMap((event) => (event.type === "stageLines" ? event.lines : []));
		if (lines.length) {
			scriptLines = lines;
			lineIndex = 0;
			prepareLinePresentation(lines);
			return true;
		}
		return false;
	}

	async function restoreCurrentSceneLines() {
		if (scriptLines.length || !gameStore.state?.currentSceneId) return;
		const sessions = await gameEngineCommands.listSessions();
		const session = sessions.find((item) => item.id === sessionId);
		const toSeq = session?.headSeq ?? 1000;
		const history: GameJournal[] = await gameEngineCommands.readJournal({
			sessionId,
			fromSeq: Math.max(0, toSeq - 1000),
			toSeq,
		});
		const savedScript = [...history]
			.sort((a, b) => a.seq - b.seq)
			.reverse()
			.map((row) => row.payloadJson as { type?: string; sceneId?: string; lines?: unknown })
			.find(
				(payload) =>
					payload.type === "sceneScript" &&
					payload.sceneId === gameStore.state?.currentSceneId &&
					Array.isArray(payload.lines),
			);
		if (Array.isArray(savedScript?.lines) && savedScript.lines.length) {
			scriptLines = savedScript.lines as StageLine[];
			lineIndex = 0;
			prepareLinePresentation(scriptLines);
		}
	}

	async function initSession() {
		try {
			isLoading = true;
			errorMessage = "";
			if (!gameStore.view) await gameStore.loadGame(gameId);
			if (gameStore.state?.sessionId !== sessionId) {
				await gameStore.continueGame(sessionId);
			}
			if (!acceptResponseLines()) syncBgmFromStage();
			await restoreCurrentSceneLines();
		} catch (error) {
			console.error("Failed to initialize game session:", error);
			errorMessage = "게임 세션을 불러오지 못했습니다.";
		} finally {
			isLoading = false;
		}
	}

	async function dispatch(command: GameCommand) {
		if (isLoading) return;
		const previousLines = scriptLines;
		const previousIndex = lineIndex;
		try {
			isLoading = true;
			errorMessage = "";
			scriptLines = [];
			lineIndex = 0;
			await gameStore.dispatchCommand(command);
			if (!acceptResponseLines()) {
				standingOverrides = {};
				syncBgmFromStage();
			}
			showActions = false;
			showMap = false;
			showInventory = false;
		} catch (error) {
			console.error("Game command failed:", error);
			scriptLines = previousLines;
			lineIndex = previousIndex;
			errorMessage = error instanceof Error ? error.message : "요청을 처리하지 못했습니다. 다시 시도해 주세요.";
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
		if (showMap) {
			showActions = false;
			showInventory = false;
		}
	}

	function toggleActions() {
		showActions = !showActions;
		if (showActions) {
			showMap = false;
			showInventory = false;
		}
	}

	function toggleInventory() {
		showInventory = !showInventory;
		if (showInventory) {
			showMap = false;
			showActions = false;
		}
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
	<Header class="topbar">
		<button class="quiet-button" onclick={handleGoBack} aria-label="뒤로 가기"><ChevronLeft size={20} /></button>
		<div class="game-heading">
			<strong>{title}</strong>
			<span>{gameStore.currentLocation?.name ?? "장소 불러오는 중"}</span>
		</div>
		<div class="top-actions">
			<div class="time-chip"><Clock3 size={15} /> {formatTime(gameStore.gameTime)}</div>
			<button class="quiet-button" onclick={() => (showBacklog = true)} aria-label="대화 기록"
				><BookOpen size={18} /></button
			>
			<button class="quiet-button" onclick={toggleInventory} aria-label="보유 아이템 보기"
				><Package size={18} /></button
			>
			<button class="quiet-button" onclick={toggleBgmMute} aria-label={isBgmMuted ? "배경음 켜기" : "배경음 끄기"} aria-pressed={isBgmMuted}
				>{#if isBgmMuted}<VolumeX size={18} />{:else}<Volume2 size={18} />{/if}</button
			>
			<button class="quiet-button" onclick={toggleMap} aria-label="지도 보기"
				><MapIcon size={18} /></button
			>
		</div>
	</Header>

	<section class="stage-area" aria-labelledby="scene-name">
		<StageView onAdvance={handleStageClick} {standingOverrides} {focusedCharacterId} />
		<div class="scene-label" id="scene-name">
			<span class="scene-dot"></span>
			{currentScene?.name ?? "현재 장면"}
		</div>

		{#if showMap}
			<aside class="map-panel" aria-labelledby="map-panel-heading">
				<div class="panel-heading">
					<div><strong id="map-panel-heading">{gameStore.currentMap?.name ?? "지도"}</strong><small>{gameStore.currentLocation?.name ?? "현재 위치 없음"}</small></div>
					<button class="quiet-button" onclick={() => (showMap = false)} aria-label="지도 닫기"><X size={17} /></button>
				</div>
				<div class="map-locations">
					{#each mapZones as zone (zone.id)}
						<section class="map-zone">
							<h3>{zone.name}</h3>
							{#each zone.locations as location (location.id)}
								<div class="map-location" class:current={location.id === gameStore.state?.locationId}>
									<span class="map-marker"></span><span>{location.name}</span>
									{#if location.id === gameStore.state?.locationId}<small>현재 위치</small>{:else if destinations.some((destination) => destination.toId === location.id)}<small>이동 경로 있음</small>{/if}
								</div>
							{/each}
						</section>
					{:else}<p class="muted">표시할 장소가 없습니다.</p>{/each}
				</div>
			</aside>
		{/if}

		{#if showInventory}
			<aside class="inventory-panel" aria-labelledby="inventory-panel-heading">
				<div class="panel-heading">
					<div><strong id="inventory-panel-heading">보유 아이템</strong><small>{inventoryEntries.length}종류</small></div>
					<button class="quiet-button" onclick={() => (showInventory = false)} aria-label="인벤토리 닫기"><X size={17} /></button>
				</div>
				{#if inventoryEntries.length}
					<div class="inventory-list">
						{#each inventoryEntries as entry (entry.itemId)}
							<article class="inventory-item">
								<div class="item-icon" aria-hidden="true">{entry.item?.icon ? "✦" : "◇"}</div>
								<div class="item-copy">
									<strong>{entry.item?.name ?? "알 수 없는 아이템"}</strong>
									{#if entry.item?.description}<p>{entry.item.description}</p>{/if}
									<div class="item-tags">
										{#if entry.item?.isKey}<span>중요</span>{/if}
										{#if entry.item?.consumable}<span>소모품</span>{/if}
									</div>
								</div>
								<span class="item-quantity">× {entry.quantity}</span>
							</article>
						{/each}
					</div>
				{:else}
					<p class="empty-inventory">아직 가지고 있는 아이템이 없습니다.</p>
				{/if}
			</aside>
		{/if}

		{#if showActions}
			<aside class="actions-panel" aria-labelledby="actions-panel-heading">
				<div class="panel-heading"><div><strong id="actions-panel-heading">이동 / 행동</strong><small>지금 실행할 수 있는 명령</small></div><button class="quiet-button" onclick={() => (showActions = false)} aria-label="행동 메뉴 닫기"><X size={17} /></button></div>
				{#if nearbyCharacters.length}
					<div class="action-group"><span class="action-label">인물과 대화</span>
						{#each nearbyCharacters as character (character.id)}<button class="destination" disabled={isLoading} onclick={() => dispatch({ type: "talk", gameCharacterId: character.id })}><span><strong>{character.name}</strong><small>대화 시작</small></span><ArrowRight size={17} /></button>{/each}
					</div>
				{/if}
				{#if localInteractables.length}
					<div class="action-group"><span class="action-label">주변 살펴보기</span>
						{#each localInteractables as item (item.id)}<button class="destination" disabled={isLoading} onclick={() => dispatch({ type: "interact", entityId: item.id })}><span><strong>{item.name}</strong><small>조사</small></span><ArrowRight size={17} /></button>{/each}
					</div>
				{/if}
				{#if canMove && destinations.length}
					<div class="action-group"><span class="action-label">이동 가능</span>
						{#each destinations as destination (destination.toId)}<button class="destination" disabled={isLoading} onclick={() => dispatch({ type: "moveTo", locationId: destination.toId })}><span><strong>{destination.location?.name}</strong><small>{Math.ceil(destination.travelSeconds / 60)}분 이동</small></span><ArrowRight size={17} /></button>{/each}
					</div>
				{/if}
				{#if choices.length}
					<div class="action-group"><span class="action-label">현재 선택지</span>
						{#each choices as choice (choice.id)}<button class="destination" disabled={isLoading} onclick={() => dispatch({ type: "choose", choiceId: choice.id })}><span><strong>{choice.text}</strong><small>선택</small></span><ArrowRight size={17} /></button>{/each}
					</div>
				{/if}
				{#if !nearbyCharacters.length && !localInteractables.length && (!canMove || !destinations.length) && !choices.length}<p class="muted">지금 실행할 수 있는 명령이 없습니다.</p>{/if}
			</aside>
		{/if}

		{#if errorMessage}<div class="error-toast" role="alert">{errorMessage}</div>{/if}

		{#if currentLine || currentScene}
			<section class="dialogue-panel" aria-live="polite">
				<button class="line-content" type="button" onclick={handleStageClick} disabled={!isReadingScript}>
					{#if !isNarration(currentLine) && speakerName}
						<span class="speaker">{speakerName}</span>
					{/if}
					<span class="dialogue-text" class:narration={isNarration(currentLine)}>
						{#if currentLine}{currentLine.text || "…"}{:else}{currentScene?.description ??
								"장면을 시작합니다."}{/if}
					</span>
					{#if scriptLines.length > 0}<span class="line-progress">{lineIndex + 1} / {scriptLines.length}</span
						>{/if}
				</button>
				{#if choices.length && !isReadingScript}
					<div class="choice-list">
						{#each choices as choice (choice.id)}
							<button
								class="choice-button"
								disabled={isLoading}
								onclick={() => dispatch({ type: "choose", choiceId: choice.id })}>{choice.text}</button
							>
						{/each}
					</div>
				{:else if isLoading}<div class="loading-label">장면을 불러오는 중…</div>{/if}
				{#if scriptLines.length > 0}
					<div class="line-navigation">
						<button class="previous-button" onclick={previousLine} disabled={isLoading || lineIndex === 0}>이전</button>
						{#if isReadingScript}
							<button class="next-button" onclick={advanceLine} disabled={isLoading}>다음 <ArrowRight size={16} /></button>
						{/if}
					</div>
				{/if}
				{#if !scriptLines.length && !isLoading}
					<p class="script-status">{choices.length ? "이 장면은 대사 대신 선택지로 진행됩니다." : "현재 씬에서 복원할 StageLine 대본을 찾지 못했습니다."}</p>
				{/if}
			</section>
		{/if}
		{#if isLoading}<div class="busy-indicator">처리 중…</div>{/if}
	</section>

	<footer class="bottom-bar">
		<span><MapPin size={15} /> {gameStore.currentLocation?.name ?? "장소 없음"}</span>
		<span>{gameStore.currentWeather?.name ?? ""}</span>
		<button class="more-button" onclick={toggleActions} aria-label="이동 및 행동 명령 보기"
			><MoreHorizontal size={18} /> 이동 / 행동</button
		>
		{#if bgmBlocked}<button class="bgm-unlock" onclick={resumeBgm}>배경음 재생</button>{/if}
	</footer>
	<audio
		class="bgm-audio"
		bind:this={bgmAudio}
		src={currentBgm ? gameStore.getAssetUrl(currentBgm.asset) : undefined}
		loop={currentBgm?.asset.settings?.loop !== false}
		preload="auto"
	></audio>
</main>

{#if showBacklog}<Backlog {sessionId} onClose={() => (showBacklog = false)} />{/if}

<style>
	.vn-player {
		height: calc(100dvh - var(--titlebar-height));
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		color: #f8f5f1;
		background: #11100f;
	}
	:global(body.is-frameless) .vn-player {
		height: 100dvh;
	}
	:global(body.is-frameless .vn-player > .header.topbar) {
		height: var(--header-height);
		flex-basis: var(--header-height);
	}
	:global(.vn-player > .header.topbar) {
		height: 62px;
		flex: 0 0 62px;
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 0 max(7rem, calc(env(safe-area-inset-right, 0px) + 7rem)) 0 max(22px, env(safe-area-inset-left, 0px));
		background: rgba(20, 18, 17, 0.94);
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		position: relative;
		top: auto;
		z-index: 31;
	}
	.quiet-button {
		display: inline-grid;
		place-items: center;
		width: 38px;
		height: 38px;
		border: 0;
		border-radius: 50%;
		color: #f2ece6;
		background: transparent;
		cursor: pointer;
	}
	.quiet-button:hover {
		background: rgba(255, 255, 255, 0.1);
	}
	.game-heading {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}
	.game-heading strong {
		font-size: 14px;
		font-weight: 650;
	}
	.game-heading span {
		color: #b4a9a0;
		font-size: 12px;
	}
	.top-actions {
		display: flex;
		align-items: center;
		gap: 7px;
		margin-left: auto;
	}
	.time-chip {
		display: flex;
		align-items: center;
		gap: 7px;
		margin-right: 8px;
		color: #e6d8ca;
		font-size: 13px;
	}
	.stage-area {
		position: relative;
		flex: 1;
		min-height: 0;
		overflow: hidden;
		background: #211f22;
		isolation: isolate;
	}
	.scene-label {
		position: absolute;
		top: 18px;
		left: 22px;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		border: 1px solid rgba(255, 255, 255, 0.16);
		border-radius: 99px;
		background: rgba(19, 17, 17, 0.55);
		backdrop-filter: blur(10px);
		color: #f7eee6;
		font-size: 12px;
	}
	.scene-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: #e3ad85;
	}
	.dialogue-panel {
		position: absolute;
		left: 50%;
		bottom: 26px;
		transform: translateX(-50%);
		width: min(880px, calc(100% - 40px));
		padding: 20px 24px 18px;
		border: 1px solid rgba(255, 255, 255, 0.2);
		border-radius: 16px;
		background: linear-gradient(145deg, rgba(23, 20, 21, 0.92), rgba(14, 14, 17, 0.91));
		box-shadow: 0 18px 60px rgba(0, 0, 0, 0.35);
		backdrop-filter: blur(14px);
		z-index: 100;
		pointer-events: auto;
	}
	.line-content {
		display: block;
		width: 100%;
		padding: 0;
		border: 0;
		color: inherit;
		text-align: left;
		background: transparent;
		cursor: pointer;
	}
	.line-content:disabled {
		cursor: default;
		opacity: 1;
	}
	.speaker {
		display: block;
		margin-bottom: 9px;
		color: #f1b78f;
		font-size: 14px;
		font-weight: 700;
	}
	.dialogue-text {
		display: block;
		margin: 0;
		min-height: 48px;
		color: #fffaf5;
		font-size: 17px;
		line-height: 1.75;
		white-space: pre-wrap;
	}
	.dialogue-text.narration {
		color: #e4ddd6;
		font-size: 15px;
	}
	.line-progress {
		display: block;
		margin-top: 8px;
		color: #b9ada4;
		font-size: 11px;
		text-align: right;
	}
	.choice-list {
		display: grid;
		gap: 9px;
		margin-top: 17px;
	}
	.choice-button {
		width: 100%;
		padding: 12px 15px;
		border: 1px solid rgba(236, 190, 158, 0.43);
		border-radius: 10px;
		text-align: left;
		color: #fffaf5;
		background: rgba(205, 143, 104, 0.14);
		cursor: pointer;
		transition:
			background 0.15s,
			transform 0.15s;
	}
	.choice-button:hover:not(:disabled) {
		background: rgba(205, 143, 104, 0.29);
		transform: translateY(-1px);
	}
	.choice-button:disabled,
	.destination:disabled {
		opacity: 0.55;
		cursor: wait;
	}
	.next-button {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 12px 0 0 auto;
		padding: 8px 12px;
		border: 0;
		border-radius: 8px;
		color: #f5c7a6;
		background: rgba(255, 255, 255, 0.08);
		cursor: pointer;
	}
	.line-navigation { display: flex; justify-content: space-between; gap: 8px; margin-top: 12px; }
	.line-navigation .next-button { margin: 0 0 0 auto; }
	.previous-button { margin-top: 0; padding: 8px 12px; border: 0; border-radius: 8px; color: #d9c9bd; background: rgba(255,255,255,.06); cursor: pointer; }
	.previous-button:disabled { opacity: .4; cursor: default; }
	.loading-label,
	.muted {
		color: #b9ada4;
		font-size: 13px;
	}
	.bottom-bar {
		display: flex;
		align-items: center;
		gap: 20px;
		height: 44px;
		flex: 0 0 44px;
		padding: 0 22px;
		background: #181615;
		border-top: 1px solid rgba(255, 255, 255, 0.07);
		color: #bdb2a9;
		font-size: 12px;
	}
	.bottom-bar span {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.more-button {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		margin-left: auto;
		padding: 6px 10px;
		border: 0;
		border-radius: 7px;
		color: #e6d8ca;
		background: rgba(255, 255, 255, 0.08);
		cursor: pointer;
	}
	.bgm-audio { display: none; }
	.bgm-unlock { padding: 6px 9px; border: 1px solid rgba(236,190,158,.3); border-radius: 7px; color: #f4d1b8; background: rgba(205,143,104,.12); cursor: pointer; }
	.map-panel,
	.actions-panel,
	.inventory-panel {
		position: absolute;
		z-index: 120;
		right: 18px;
		width: min(320px, calc(100% - 36px));
		padding: 14px;
		border: 1px solid rgba(255, 255, 255, 0.14);
		border-radius: 14px;
		background: rgba(25, 22, 22, 0.96);
		box-shadow: 0 14px 45px #0008;
		backdrop-filter: blur(14px);
	}
	.map-panel { top: 16px; }
	.inventory-panel { top: 16px; }
	.actions-panel { bottom: 16px; max-height: calc(100% - 32px); overflow-y: auto; }
	.panel-heading {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding-left: 7px;
		margin-bottom: 8px;
		font-size: 14px;
	}
	.panel-heading > div { display: flex; flex-direction: column; gap: 5px; }
	.panel-heading small { color: #aa9d94; font-size: 12px; }
	.map-locations { display: grid; gap: 13px; max-height: min(60vh, 520px); overflow-y: auto; padding: 4px; }
	.map-zone h3 { margin: 0 0 7px; color: #b9ada4; font-size: 11px; text-transform: uppercase; letter-spacing: .08em; }
	.map-location { display: flex; align-items: center; gap: 9px; min-height: 35px; padding: 7px 9px; border-radius: 8px; color: #eee5de; font-size: 13px; }
	.map-location.current { background: rgba(205, 143, 104, .18); color: #ffd9bd; }
	.map-location small { margin-left: auto; color: #b9ada4; font-size: 10px; }
	.map-marker { width: 8px; height: 8px; flex: 0 0 8px; border: 1px solid #c6a48e; border-radius: 50%; }
	.map-location.current .map-marker { border-color: #f1b78f; background: #f1b78f; box-shadow: 0 0 9px #f1b78f88; }
	.action-group { display: grid; gap: 2px; margin-top: 10px; }
	.action-label { padding: 5px 9px; color: #b9ada4; font-size: 11px; }
	.inventory-list { display: grid; gap: 7px; max-height: min(62vh, 540px); overflow-y: auto; padding: 3px; }
	.inventory-item { display: flex; align-items: flex-start; gap: 10px; padding: 10px 8px; border: 1px solid rgba(255,255,255,.07); border-radius: 10px; background: rgba(255,255,255,.035); }
	.item-icon { display: grid; place-items: center; width: 34px; height: 34px; flex: 0 0 34px; border-radius: 9px; color: #f1b78f; background: rgba(205,143,104,.16); font-size: 18px; }
	.item-copy { flex: 1; min-width: 0; }
	.item-copy > strong { display: block; color: #fff5ed; font-size: 13px; font-weight: 650; }
	.item-copy p { margin-top: 5px; color: #b9ada4; font-size: 11px; line-height: 1.5; }
	.item-tags { display: flex; gap: 5px; margin-top: 6px; }
	.item-tags span { padding: 3px 6px; border-radius: 99px; color: #e8bea2; background: rgba(205,143,104,.13); font-size: 10px; }
	.item-quantity { flex: 0 0 auto; color: #f4d1b8; font-size: 12px; font-weight: 650; }
	.empty-inventory { padding: 18px 8px; color: #b9ada4; font-size: 13px; line-height: 1.5; text-align: center; }
	.destination {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: 11px 9px;
		border: 0;
		border-radius: 9px;
		color: #f7eee6;
		background: transparent;
		text-align: left;
		cursor: pointer;
	}
	.destination:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.09);
	}
	.destination span {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.destination small {
		color: #aa9d94;
	}
	.error-toast {
		position: absolute;
		z-index: 5;
		top: 18px;
		left: 50%;
		transform: translateX(-50%);
		padding: 10px 15px;
		border: 1px solid #e99b8b66;
		border-radius: 9px;
		color: #ffe0d9;
		background: #492722ee;
		font-size: 13px;
	}
	.script-status { margin-top: 12px; color: #d2bba9; font-size: 12px; line-height: 1.5; }
	.busy-indicator {
		position: absolute;
		top: 18px;
		right: 22px;
		color: #f4d1b8;
		font-size: 12px;
	}
	@media (max-width: 620px) {
		:global(.vn-player > .header.topbar) {
			padding: 0 max(7rem, calc(env(safe-area-inset-right, 0px) + 7rem)) 0 max(10px, env(safe-area-inset-left, 0px));
			gap: 7px;
		}
		.top-actions {
			gap: 0;
		}
		.time-chip {
			margin-right: 0;
		}
		.scene-label {
			left: 12px;
			top: 12px;
		}
		.dialogue-panel {
			bottom: 12px;
			width: calc(100% - 20px);
			padding: 16px;
		}
		.dialogue-text {
			font-size: 15px;
		}
		.bottom-bar {
			padding: 0 12px;
			gap: 12px;
		}
	}
</style>
