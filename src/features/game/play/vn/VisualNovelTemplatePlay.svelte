<script lang="ts">
	import { onMount, tick } from "svelte";
	import type { AssetRef, GameCommand, GameJournal, NpcReply, StageLine } from "$features/game/types";
	import { call, commands } from "$shared/ipc/client";
	import { layout } from "$app/layout.svelte";
	import "../stage.css";
	import StageView from "./StageView.svelte";
	import PlayInfo from "./PlayInfo.svelte";
	import { playBridge } from "../../runtime/playBridge.svelte";
	import { panel } from "$app/panel.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import DialoguePanel from "./DialoguePanel.svelte";
	import ItemGainNotices from "./ItemGainNotices.svelte";
	import {
		createLocationArrivalLine,
		getItemAddedEvents,
		getStageLines,
		normalizeCueType,
		type ItemGainNotice,
	} from "./playUtils";
	import { gameStore } from "$features/game/runtime/gameStore.svelte";
	import { assetUrl } from "$features/game/runtime/gameStore.svelte";
	import { runtimeStore } from "$features/game/runtime/runtimeStore.svelte";

	type Props = { gameId: string; sessionId: string };
	let { gameId, sessionId }: Props = $props();
	type DisplayStanding = { emotion: string; asset: AssetRef };
	type CurrentBgm = { mood: string; asset: AssetRef };

	let isLoading = $state(false);
	// ---- 대사창 아래 동작 ----
	let uiHidden = $state(false);
	let autoAdvance = $state(false);
	let showWaitMenu = $state(false);
	let scriptLines = $state<StageLine[]>([]);
	let lineIndex = $state(0);
	let scriptFinished = $state(false);
	let arrivalOnly = $state(false);
	let standingOverrides = $state<Record<string, DisplayStanding | null>>({});
	let standingBaseline: Record<string, DisplayStanding | null> = {};
	let currentBgm = $state<CurrentBgm | null>(null);
	let scriptStartBgm: CurrentBgm | null = null;
	let bgmBlocked = $state(false);
	let isBgmMuted = $state(false);
	let bgmAudio: HTMLAudioElement;
	let itemGainNotices = $state<ItemGainNotice[]>([]);
	let nextItemGainNoticeId = 0;
	let npcResponseChoices = $state<string[]>([]);
	let hideSceneChoicesDuringTalk = $state(false);
	let freeTextInput = $state("");

	const currentLine = $derived(scriptLines[lineIndex] ?? null);
	const choices = $derived(hideSceneChoicesDuringTalk ? [] : (runtimeStore.currentChoices ?? []));
	const isReadingScript = $derived(scriptLines.length > 0 && !scriptFinished);
	const currentCharacter = $derived(
		currentLine?.speaker
			? gameStore.view?.characters.find((character) => character.id === currentLine.speaker)
			: null,
	);
	const speakerName = $derived(currentLine?.speaker === "player" ? "나" : (currentCharacter?.name ?? null));
	const focusedCharacterId = $derived(currentCharacter?.id ?? null);
	const freeTextCharacterId = $derived.by(() => {
		const presentNpcIds = runtimeStore.state?.presentNpcs ?? [];
		const activeNpcId = runtimeStore.state?.activeNpcId;
		if (activeNpcId && presentNpcIds.includes(activeNpcId)) return activeNpcId;
		if (currentLine?.speaker && currentLine.speaker !== "player" && presentNpcIds.includes(currentLine.speaker)) {
			return currentLine.speaker;
		}
		return presentNpcIds[0] ?? null;
	});
	const activeNpcId = $derived(runtimeStore.state?.activeNpcId ?? null);
	const title = $derived(gameStore.view?.game?.title ?? "게임");

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
		void bgmAudio
			.play()
			.then(() => (bgmBlocked = false))
			.catch(() => (bgmBlocked = true));
	}

	function toggleBgmMute() {
		isBgmMuted = !isBgmMuted;
		if (bgmAudio) bgmAudio.muted = isBgmMuted;
	}

	$effect(() => {
		if (bgmAudio) bgmAudio.muted = isBgmMuted;
	});

	function syncBgmFromStage() {
		const bgm = runtimeStore.stage?.bgm as unknown as { mood?: string; asset?: AssetRef | null } | undefined;
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
				if (normalizeCueType(cue.type) === "bgm") hasBgmCue = true;
				if (["standing", "standing_off"].includes(normalizeCueType(cue.type)) && cue.characterId)
					affectedCharacters.add(cue.characterId);
			}
		}
		const baseline: Record<string, DisplayStanding | null> = { ...(runtimeStore.stage?.standings ?? {}) };
		for (const characterId of affectedCharacters) {
			const normal = gameStore.view?.assets.characters.find(
				(asset) =>
					asset.characterId === characterId &&
					asset.usage === "standing" &&
					asset.emotion.toLowerCase() === "normal",
			);
			baseline[characterId] = normal ? { emotion: normal.emotion, asset: normal } : null;
		}
		if (!hasBgmCue) syncBgmFromStage();
		standingBaseline = baseline;
		scriptStartBgm = currentBgm ? { ...currentBgm } : null;
		rebuildLinePresentation(0);
	}

	function acceptResponseLines(prefixLines: StageLine[] = []) {
		const responseLines = getStageLines(runtimeStore?.events ?? []);
		arrivalOnly = prefixLines.length > 0 && responseLines.length === 0;
		const lines = [...prefixLines, ...responseLines];
		if (!lines.length) {
			scriptFinished = true;
			return false;
		}
		scriptLines = lines;
		lineIndex = 0;
		scriptFinished = false;
		prepareLinePresentation(lines);
		return true;
	}

	async function restoreCurrentSceneLines() {
		if (scriptLines.length || !runtimeStore?.currentSceneId) return;
		const sessions = await call(commands.listSessions(), { source: "game", silent: true });
		const session = sessions.find((item) => item.id === sessionId);
		const toSeq = session?.headSeq ?? 1000;
		const history = (await call(commands.readJournal(sessionId, Math.max(0, toSeq - 1000), toSeq), { source: "game", silent: true })) as unknown as GameJournal[];
		const savedScript = [...history]
			.sort((a, b) => a.seq - b.seq)
			.reverse()
			.map((row) => row.payloadJson as { type?: string; sceneId?: string; lines?: unknown })
			.find(
				(payload) =>
					payload.type === "sceneScript" &&
					payload.sceneId === runtimeStore?.currentSceneId &&
					Array.isArray(payload.lines),
			);
		if (Array.isArray(savedScript?.lines) && savedScript.lines.length) {
			arrivalOnly = false;
			scriptLines = savedScript.lines as StageLine[];
			lineIndex = 0;
			scriptFinished = false;
			prepareLinePresentation(scriptLines);
		}
	}

	async function initSession() {
		try {
			isLoading = true;
			if (!gameStore.view) await gameStore.loadBundle(gameId);
			if (runtimeStore?.state?.sessionId !== sessionId) await runtimeStore.loadState(sessionId);
			if (!acceptResponseLines()) syncBgmFromStage();
			await restoreCurrentSceneLines();
		} catch (error) {
			console.error("Failed to initialize game session:", error);
		} finally {
			isLoading = false;
		}
	}

	function showItemGains() {
		const addedItems = getItemAddedEvents(runtimeStore?.events ?? []);
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
		const previousScriptFinished = scriptFinished;
		const previousArrivalOnly = arrivalOnly;
		try {
			isLoading = true;
			scriptLines = [];
			lineIndex = 0;
			scriptFinished = false;
			arrivalOnly = false;
			await runtimeStore.dispatchCommand(command);
			npcResponseChoices = [];
			if (command.type === "talk") {
				hideSceneChoicesDuringTalk = false;
				scriptFinished = true;
			} else if (command.type === "stopTalk") {
				hideSceneChoicesDuringTalk = false;
				npcResponseChoices = [];
				freeTextInput = "";
				scriptFinished = true;
			}
			showItemGains();
			const arrivalEvent =
				command.type === "moveTo"
					? runtimeStore?.events?.find((event) => event.type === "locationChanged")
					: undefined;
			const arrivalLine =
				arrivalEvent?.type === "locationChanged"
					? createLocationArrivalLine(
							gameStore.view?.locations.find((location) => location.id === arrivalEvent.to),
						)
					: null;
			if (!acceptResponseLines(arrivalLine ? [arrivalLine] : [])) {
				standingOverrides = {};
				syncBgmFromStage();
			}
			showWaitMenu = false;
		} catch (error) {
			console.error("Game command failed:", error);
			scriptLines = previousLines;
			lineIndex = previousIndex;
			scriptFinished = previousScriptFinished;
			arrivalOnly = previousArrivalOnly;
		} finally {
			isLoading = false;
		}
	}

	async function submitNpcInput(text = freeTextInput) {
		const input = text.trim();
		const characterId = freeTextCharacterId;
		if (!input || !characterId || isLoading) return;

		try {
			isLoading = true;
			freeTextInput = "";
			npcResponseChoices = [];
			hideSceneChoicesDuringTalk = true;
			if (runtimeStore?.activeNpcId !== characterId) {
				await runtimeStore.dispatchCommand({ type: "talk", gameCharacterId: characterId });
			}
			const reply = (await call(commands.npcSay(characterId, input), { source: "game" })) as unknown as NpcReply;
			runtimeStore.setState({
				state: reply.state,
				events: reply.events,
				history: { entries: [], hasMore: false, oldestSeq: 0 },
			});
			scriptLines = reply.lines;
			lineIndex = 0;
			scriptFinished = scriptLines.length === 0;
			arrivalOnly = false;
			npcResponseChoices = reply.events.flatMap((event) =>
				event.type === "choicePresented" ? event.choices : [],
			);
			if (scriptLines.length) prepareLinePresentation(scriptLines);
			else syncBgmFromStage();
		} catch (error) {
			console.error("NPC message failed:", error);
			freeTextInput = input;
			hideSceneChoicesDuringTalk = false;
		} finally {
			isLoading = false;
		}
	}

	function handleChoice(choiceId: string) {
		const choice = runtimeStore.currentChoices.find((candidate) => candidate.id === choiceId);
		if (!choice) return;
		// 대화 중 씬 종료 화면에서 고른 선택지는 플레이어의 다음 발화로 NPC에게 전달한다.
		if (scriptFinished && activeNpcId) {
			void submitNpcInput(choice.text);
			return;
		}
		void dispatch({ type: "choose", choiceId });
	}

	function advanceLine() {
		if (lineIndex < scriptLines.length - 1) {
			lineIndex += 1;
			rebuildLinePresentation(lineIndex);
		} else if (arrivalOnly) {
			arrivalOnly = false;
			scriptLines = [];
			lineIndex = 0;
		} else if (scriptLines.length > 0) {
			scriptFinished = true;
		}
	}

	function previousLine() {
		if (scriptFinished) {
			scriptFinished = false;
			rebuildLinePresentation(lineIndex);
			return;
		}
		if (lineIndex > 0) {
			lineIndex -= 1;
			scriptFinished = false;
			rebuildLinePresentation(lineIndex);
		}
	}

	function handleStageClick() {
		if (uiHidden) return void (uiHidden = false);
		if (!isLoading) advanceLine();
	}

	/** 지도·이동/행동·소지품은 오른쪽 패널 탭으로 */
	function openPanelTab(id: string) {
		panel.tab = id;
		if (!panel.open) panel.toggle();
	}

	/** 자동 넘김: 대사 길이에 맞춰 잠시 뒤 다음 줄 (선택지·입력에서 멈춤) */
	$effect(() => {
		if (!autoAdvance || !isReadingScript || isLoading || uiHidden) return;
		const len = currentLine?.text?.length ?? 0;
		const t = setTimeout(advanceLine, Math.min(8000, 1600 + len * 45));
		return () => clearTimeout(t);
	});
	/** 이 장면 대사를 끝까지 넘김 (선택지·입력이 나오는 곳까지) */
	function skipScript() {
		for (let i = 0; i < 500 && isReadingScript; i++) advanceLine();
	}
	function copyLine() {
		const text = currentLine?.text ?? runtimeStore.currentScene?.description ?? "";
		if (!text) return;
		const who = speakerName ? `${speakerName}: ` : "";
		void navigator.clipboard.writeText(who + text).then(
			() => toasts.success("대사 복사함"),
			() => toasts.error("복사 못 함"),
		);
	}

	function toggleWaitMenu() {
		showWaitMenu = !showWaitMenu;
	}

	// 오른쪽 패널(이동·행동, 지도)이 이 화면의 dispatch로 명령을 보냄
	$effect(() => {
		playBridge.dispatch = dispatch;
		return () => {
			if (playBridge.dispatch === dispatch) playBridge.dispatch = null;
		};
	});
	$effect(() => {
		playBridge.loading = isLoading;
	});

	onMount(initSession);

	// ---- 키보드: Space/Enter/→ 다음, ← 이전, 1~9 선택지, Esc 패널 닫기, M 지도, I 소지품, L 지난 대사 ----
	function closePanels() {
		showWaitMenu = false;
		uiHidden = false;
	}
	function onKey(e: KeyboardEvent) {
		const t = e.target as HTMLElement | null;
		if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
		if (e.ctrlKey || e.metaKey || e.altKey) return;
		if (e.key === "Escape") return closePanels();
		if (e.key === " " || e.key === "Enter" || e.key === "ArrowRight") {
			if (isReadingScript) {
				e.preventDefault();
				handleStageClick();
			}
		} else if (e.key === "ArrowLeft") {
			e.preventDefault();
			previousLine();
		} else if (/^[1-9]$/.test(e.key) && scriptFinished) {
			const i = Number(e.key) - 1;
			if (i < choices.length) handleChoice(choices[i].id);
			else if (i - choices.length < npcResponseChoices.length) void submitNpcInput(npcResponseChoices[i - choices.length]);
		} else if (e.key === "m") openPanelTab("map");
		else if (e.key === "i") openPanelTab("items");
		else if (e.key === "a") openPanelTab("actions");
		else if (e.key === "l") openPanelTab("log");
		else if (e.key === "h") uiHidden = !uiHidden;
	}
	const focusMode = $derived(!layout.sidebar);
</script>

<svelte:window onkeydown={onKey} />

<main class="vn-player game-theme">

	<section class="stage-area" aria-labelledby="scene-name">
		<StageView onAdvance={handleStageClick} {standingOverrides} {focusedCharacterId} />
		{#if !uiHidden}
			<PlayInfo
				sceneName={runtimeStore.currentScene?.name ?? title}
				locationName={runtimeStore.currentLocation?.name ?? "장소 불러오는 중"}
				weatherName={runtimeStore.currentWeather?.name ?? ""}
				gameTime={runtimeStore.gameTime}
				canWait={runtimeStore.canWait}
				{isLoading}
				{showWaitMenu}
				{bgmBlocked}
				onToggleWaitMenu={toggleWaitMenu}
				onWait={(hours) => dispatch({ type: "advanceTime", minutes: hours * 60 })}
				onResumeBgm={resumeBgm}
			/>
		{/if}
		<ItemGainNotices notices={itemGainNotices} />
		<DialoguePanel
			show={!uiHidden && Boolean(currentLine || runtimeStore.currentScene || activeNpcId || scriptFinished)}
			line={currentLine}
			{speakerName}
			sceneDescription={runtimeStore.currentScene?.description ?? null}
			{choices}
			npcChoices={npcResponseChoices}
			canSendInput={Boolean(activeNpcId)}
			canStopTalk={Boolean(activeNpcId)}
			inputValue={freeTextInput}
			{lineIndex}
			lineCount={scriptLines.length}
			{scriptFinished}
			{isReadingScript}
			{isLoading}
			onStageClick={handleStageClick}
			onPrevious={previousLine}
			onAdvance={advanceLine}
			onChoose={handleChoice}
			onNpcChoice={(text) => submitNpcInput(text)}
			onInputChange={(value) => (freeTextInput = value)}
			onSubmitInput={() => submitNpcInput()}
			onStopTalk={() => dispatch({ type: "stopTalk" })}
			{autoAdvance}
			bgmMuted={isBgmMuted}
			{focusMode}
			onCopy={copyLine}
			onToggleAuto={() => (autoAdvance = !autoAdvance)}
			onSkip={skipScript}
			onHide={() => (uiHidden = true)}
			onLog={() => openPanelTab("log")}
			onToggleMute={toggleBgmMute}
			onToggleFocus={() => layout.toggleSidebar()}
		/>
		{#if uiHidden}<p class="hidden-hint">무대를 누르면 대사창이 다시 보임</p>{/if}
		{#if isLoading}<div class="busy" role="status"><span></span>처리 중</div>{/if}
	</section>

	<audio
		class="bgm-audio"
		bind:this={bgmAudio}
		src={currentBgm ? assetUrl(currentBgm.asset.path) : undefined}
		loop={currentBgm?.asset.settings?.loop !== false}
		preload="auto"
	></audio>
</main>

<style>
	.vn-player {
		height: 100%;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		color: var(--game-text-main);
		background: var(--game-player-bg);
	}
	.stage-area {
		position: relative;
		flex: 1;
		min-height: 0;
		overflow: hidden;
		background: var(--game-stage-bg);
		isolation: isolate;
	}
	.hidden-hint {
		position: absolute;
		z-index: 30;
		left: 50%;
		bottom: 16px;
		transform: translateX(-50%);
		margin: 0;
		padding: 6px 12px;
		border-radius: 999px;
		background: var(--game-scene-label-bg);
		color: var(--game-text-muted);
		font-size: 12px;
		pointer-events: none;
	}
	.bgm-audio {
		display: none;
	}
	.busy {
		position: absolute;
		z-index: 150;
		top: 16px;
		left: 50%;
		transform: translateX(-50%);
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 12px;
		border: 1px solid var(--game-border);
		border-radius: 999px;
		background: var(--game-scene-label-bg);
		color: var(--game-accent-strong);
		font-size: 12px;
		backdrop-filter: blur(10px);
	}
	.busy span {
		width: 10px;
		height: 10px;
		border: 2px solid var(--game-accent-border);
		border-top-color: var(--game-accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
