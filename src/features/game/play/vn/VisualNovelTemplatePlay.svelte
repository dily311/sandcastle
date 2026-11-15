<script lang="ts">
	import { onMount, tick } from "svelte";
	import { Channel } from "@tauri-apps/api/core";
	import type { StreamPayload } from "$ipc/bindings";
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
	import Backlog from "./Backlog.svelte";
	import {
		createLocationArrivalLine,
		directionParticle,
		foldDirectionLines,
		getStageLines,
		isDirectionOnly,
		itemGainLines,
		normalizeCueType,
		type PlayAction,
	} from "./playUtils";
	import { gameStore } from "$features/game/runtime/gameStore.svelte";
	import { assetUrl } from "$features/game/runtime/gameStore.svelte";
	import { runtimeStore } from "$features/game/runtime/runtimeStore.svelte";
	import { reportGameError } from "$features/game/errors";

	type Props = { gameId: string; sessionId: string };
	let { gameId, sessionId }: Props = $props();
	type DisplayStanding = { emotion: string; asset: AssetRef };
	type CurrentBgm = { mood: string; asset: AssetRef };

	let isLoading = $state(false);
	/** AI 응답을 기다리는 동안 받은 글 (태그는 백엔드가 걸러 보냄). 응답이 오면 비운다 */
	let streamingText = $state("");
	// ---- 대사창 아래 동작 ----
	let uiHidden = $state(false);
	let autoAdvance = $state(false);
	let showWaitMenu = $state(false);
	/** 지난 대사 창 (L) */
	let backlogOpen = $state(false);
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
	const activeNpcId = $derived(runtimeStore.state?.activeNpcId ?? null);
	/** 열린 씬이 있는가. 씬 동안은 1:1 대화(talk)가 막히고 입력은 씬 입력(sceneSay)으로 간다 */
	const sceneOpen = $derived(Boolean(runtimeStore.currentSceneId));
	/** 씬 입력·씬 끝내기를 쓸 수 있는가 (엔딩 씬이나 AI 이어가기가 꺼진 씬이면 false) */
	const sceneChat = $derived(Boolean(runtimeStore.state?.sceneChat));
	type SayTarget = { kind: "scene" } | { kind: "npc"; characterId: string };
	/** 할 일이 없을 때 대사창에 바로 보여줄 이동·행동 (오른쪽 패널의 이동·행동과 같은 목록) */
	const playActions = $derived.by<PlayAction[]>(() => {
		if (sceneOpen || activeNpcId) return [];
		const out: PlayAction[] = [];
		for (const c of runtimeStore.nearbyCharacters) out.push({ id: `talk:${c.id}`, kind: "talk", label: `${c.name}에게 말 걸기`, run: () => dispatch({ type: "talk", gameCharacterId: c.id }) });
		for (const i of runtimeStore.localInteractables) out.push({ id: `look:${i.id}`, kind: "look", label: `${i.name} 살펴보기`, run: () => dispatch({ type: "interact", entityId: i.id }) });
		for (const d of runtimeStore.destinations) {
			const locked = Boolean(d.requirement && !d.requirement.available);
			out.push({
				id: `move:${d.toId}`,
				kind: "move",
				label: `${d.location?.name ?? "어딘가"}${directionParticle(d.location?.name ?? "어딘가")} 가기`,
				hint: locked ? "조건이 필요함" : `${Math.max(1, Math.ceil(d.travelSeconds / 60))}분`,
				disabled: locked,
				run: () => dispatch({ type: "moveTo", locationId: d.toId }),
			});
		}
		return out;
	});
	/** 입력·AI 제안 선택지를 보낼 곳. 없으면 입력창을 숨김 */
	const sayTarget = $derived.by<SayTarget | null>(() => {
		if (sceneOpen) return sceneChat ? { kind: "scene" } : null;
		return activeNpcId ? { kind: "npc", characterId: activeNpcId } : null;
	});
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

	// ---- 효과음·음성: 줄이 처음 보일 때 한 번만 (이전 대사로 돌아가거나 다시 그릴 때는 다시 안 냄) ----
	const playedLines = new WeakSet<StageLine>();
	let voiceAudio: HTMLAudioElement | null = null;
	function audioVolume(asset: AssetRef, fallback: number) {
		const v = Number(asset.settings?.volume);
		return Number.isFinite(v) ? Math.max(0, Math.min(1, v)) : fallback;
	}
	function playLineAudio(line: StageLine) {
		if (isBgmMuted) return;
		for (const rawCue of line.cues ?? []) {
			const cue = rawCue as unknown as { type?: string; asset?: AssetRef | null };
			const kind = normalizeCueType(cue.type);
			if ((kind !== "sfx" && kind !== "voice") || !cue.asset) continue;
			const audio = new Audio(assetUrl(cue.asset.path));
			audio.volume = audioVolume(cue.asset, kind === "voice" ? 1 : 0.8);
			if (kind === "voice") {
				// 음성은 한 번에 하나: 다음 대사의 음성이 오면 앞의 것을 끊는다
				voiceAudio?.pause();
				voiceAudio = audio;
			}
			void audio.play().catch(() => {
				/* 자동 재생이 막히면 조용히 넘김 (배경음 버튼으로 소리를 켠 뒤부터 들림) */
			});
		}
	}
	$effect(() => {
		const line = currentLine;
		if (!line || scriptFinished || playedLines.has(line)) return;
		playedLines.add(line);
		playLineAudio(line);
	});

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
		if (isBgmMuted) voiceAudio?.pause();
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
		const events = runtimeStore?.events ?? [];
		// 얻은 아이템은 대사창에 "~을 얻었다"로 이어서 보여준다
		const responseLines = [...getStageLines(events), ...itemGainLines(events, gameStore.view?.items)];
		arrivalOnly = prefixLines.length > 0 && responseLines.every(isDirectionOnly);
		const lines = foldDirectionLines([...prefixLines, ...responseLines]);
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
			scriptLines = foldDirectionLines(savedScript.lines as StageLine[]);
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


	async function dispatch(command: GameCommand) {
		stageFrom = null;
		editing = false;
		if (isLoading) return;
		// 선택지 문구는 고르기 전에 읽어 둔다 (고른 뒤에는 씬이 바뀔 수 있다)
		const choiceText = command.type === "choose" ? runtimeStore.currentChoices.find((c) => c.id === command.choiceId)?.text : undefined;
		let continueWith: string | null = null;
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
			const continues = (runtimeStore.events ?? []).some((e) => e.type === "choiceMade" && e.continuesScene);
			if (continues && choiceText && runtimeStore.state?.sceneChat) continueWith = choiceText;
		} catch (error) {
			console.error("Game command failed:", error);
			scriptLines = previousLines;
			lineIndex = previousIndex;
			scriptFinished = previousScriptFinished;
			arrivalOnly = previousArrivalOnly;
		} finally {
			isLoading = false;
		}
		// 씬이 이어지는 선택지: 선택지의 결과 대사 뒤에 AI 서술을 붙인다
		if (continueWith) await say(continueWith, { keepLines: true, fromChoice: true });
	}

	/**
	 * 플레이어의 말·행동을 보낸다.
	 * - 씬이 열려 있으면 서술자에게(sceneSay): AI가 장면 전체를 이어 간다. 씬 선택지는 계속 보인다
	 * - 아니면 대화 중인 NPC에게(npcSay): 대화 동안 씬 선택지는 숨긴다
	 * keepLines: 지금 무대의 대사(선택지 결과 등) 뒤에 응답을 붙인다
	 * fromChoice: 선택지에서 온 문구라 실패해도 입력창에 되돌리지 않는다
	 */
	async function say(text = freeTextInput, opts: { keepLines?: boolean; fromChoice?: boolean } = {}) {
		const input = text.trim();
		const target = sayTarget;
		if (!input || !target || isLoading) return;
		const prefix = opts.keepLines ? scriptLines : [];
		const startAt = opts.keepLines ? lineIndex : 0;
		try {
			isLoading = true;
			editing = false;
			if (!opts.fromChoice) freeTextInput = "";
			npcResponseChoices = [];
			if (target.kind === "npc") hideSceneChoicesDuringTalk = true;
			const channel = new Channel<StreamPayload>();
			channel.onmessage = (p) => (streamingText += p.chunk);
			const request = target.kind === "scene" ? commands.sceneSay(input, channel) : commands.npcSay(target.characterId, input, channel);
			const reply = (await call(request, { source: "game", silent: true }).catch((e) => {
				reportGameError(e, gameStore.project?.id);
				throw e;
			})) as unknown as NpcReply;
			applyReply(reply, prefix, startAt);
		} catch (error) {
			console.error(target.kind === "scene" ? "Scene input failed:" : "NPC message failed:", error);
			if (!opts.fromChoice) freeTextInput = input;
			hideSceneChoicesDuringTalk = false;
		} finally {
			streamingText = "";
			isLoading = false;
		}
	}

	/** AI 응답(새로 받은 것, 다시 만든 것)을 무대에 올림. prefix가 있으면 그 뒤에 붙이고 startAt 줄부터 보여준다 */
	function applyReply(reply: NpcReply, prefix: StageLine[] = [], startAt = 0) {
		runtimeStore.setState({
			state: reply.state,
			events: reply.events,
			history: { entries: [], hasMore: false, oldestSeq: 0 },
		});
		// 씬 입력의 응답으로 씬이 끝났으면 씬 선택지를 다시 숨길 이유가 없다
		if (!reply.state.currentSceneId && !reply.state.activeNpcId) hideSceneChoicesDuringTalk = false;
		scriptLines = [...prefix, ...foldDirectionLines([...reply.lines, ...itemGainLines(reply.events, gameStore.view?.items)])];
		lineIndex = Math.min(startAt, Math.max(0, scriptLines.length - 1));
		scriptFinished = scriptLines.length === 0;
		arrivalOnly = false;
		npcResponseChoices = reply.events.flatMap((event) => (event.type === "choicePresented" ? event.choices : []));
		lastReply = reply.messageId ? { messageId: reply.messageId, text: reply.text, lines: reply.lines } : null;
		stageFrom = reply.messageId ?? null;
		if (scriptLines.length) {
			prepareLinePresentation(scriptLines);
			if (lineIndex > 0) rebuildLinePresentation(lineIndex);
		} else syncBgmFromStage();
	}

	// ---- 메시지 고치기 · 다시 만들기 (화면은 대사지만 단위는 AI 메시지 하나) ----
	let lastReply = $state<{ messageId: string; text: string; lines: StageLine[] } | null>(null);
	/** 지금 무대의 대사가 어느 AI 메시지의 것인지 (명령·선택지로 다른 대사가 오르면 비움) */
	let stageFrom = $state<string | null>(null);
	const onReply = $derived(!!lastReply && stageFrom === lastReply.messageId);
	let editing = $state(false);
	let editDraft = $state("");
	function startEdit() {
		if (!lastReply) return;
		editDraft = lastReply.text;
		editing = true;
		autoAdvance = false;
	}
	async function saveEdit() {
		if (!lastReply || !editDraft.trim()) return;
		isLoading = true;
		try {
			const r = await call(commands.editGameMessage(lastReply.messageId, editDraft), { source: "game" });
			const lines = foldDirectionLines(r.lines as unknown as StageLine[]);
			lastReply = { messageId: r.id, text: r.text, lines };
			scriptLines = lines;
			lineIndex = 0;
			scriptFinished = lines.length === 0;
			if (lines.length) prepareLinePresentation(lines);
			editing = false;
			toasts.success("메시지 고침");
		} finally {
			isLoading = false;
		}
	}
	async function regenerate() {
		if (!lastReply || isLoading) return;
		isLoading = true;
		editing = false;
		try {
			const reply = (await call(commands.regenerateGameReply(lastReply.messageId), { source: "game" })) as unknown as NpcReply;
			applyReply(reply);
		} finally {
			isLoading = false;
		}
	}

	function handleChoice(choiceId: string) {
		if (!runtimeStore.currentChoices.some((candidate) => candidate.id === choiceId)) return;
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
		if (backlogOpen) return; // 창이 Esc·L을 직접 받는다
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
			else if (i - choices.length < npcResponseChoices.length) void say(npcResponseChoices[i - choices.length], { fromChoice: true });
		} else if (e.key === "m") openPanelTab("map");
		else if (e.key === "i") openPanelTab("items");
		else if (e.key === "a") openPanelTab("actions");
		else if (e.key === "l") backlogOpen = !backlogOpen;
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
		<DialoguePanel
			show={!uiHidden && Boolean(currentLine || runtimeStore.currentScene || activeNpcId || scriptFinished)}
			line={currentLine}
			{speakerName}
			sceneDescription={runtimeStore.currentScene?.description ?? null}
			{choices}
			npcChoices={npcResponseChoices}
			canSendInput={Boolean(sayTarget)}
			inputPlaceholder={sayTarget?.kind === "scene" ? "이 장면에서 할 말이나 행동 입력" : "하고 싶은 말 입력"}
			canStopTalk={!sceneOpen && Boolean(activeNpcId)}
			actions={playActions}
			canEndScene={sceneOpen && sceneChat}
			inputValue={freeTextInput}
			{lineIndex}
			lineCount={scriptLines.length}
			{scriptFinished}
			{isReadingScript}
			{isLoading}
			{streamingText}
			onStageClick={handleStageClick}
			onPrevious={previousLine}
			onAdvance={advanceLine}
			onChoose={handleChoice}
			onNpcChoice={(text) => say(text, { fromChoice: true })}
			onInputChange={(value) => (freeTextInput = value)}
			onSubmitInput={() => say()}
			onStopTalk={() => dispatch({ type: "stopTalk" })}
			onEndScene={() => dispatch({ type: "endScene" })}
			{autoAdvance}
			bgmMuted={isBgmMuted}
			{focusMode}
			onCopy={copyLine}
			onToggleAuto={() => (autoAdvance = !autoAdvance)}
			onSkip={skipScript}
			onHide={() => (uiHidden = true)}
			onLog={() => (backlogOpen = true)}
			onToggleMute={toggleBgmMute}
			onToggleFocus={() => layout.toggleSidebar()}
			canEditMessage={onReply && !isLoading}
			{editing}
			{editDraft}
			onEdit={startEdit}
			onEditInput={(v) => (editDraft = v)}
			onEditSave={saveEdit}
			onEditCancel={() => (editing = false)}
			onRegenerate={regenerate}
		/>
		{#if backlogOpen}<Backlog onclose={() => (backlogOpen = false)} />{/if}
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
