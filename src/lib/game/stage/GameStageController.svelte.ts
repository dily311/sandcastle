import * as PIXI from "pixi.js";
import { GameHost } from "../host/gameHost";
import { gameCommands } from "$ipc/commands";
import { NpcConversationTracker, callNpcStructured, addSwipe, selectSwipe } from "../npc/gameLogging";
import type { NpcResponder } from "../npc/npcResponder";
import type { GameState, StateChangeInstruction } from "../types";
import type { Message } from "$/lib/type/conv";

export interface GameStageControllerOptions {
	gameId: string;
	startSceneKey: string;
	slotNumber: number;

	npcResponder: NpcResponder;

	canvasHost: HTMLDivElement;
	iframeEl: HTMLIFrameElement;

	onDisplay: (display: GameStageDisplay | null) => void;
	onDebug: (message: string) => void;
	onLogUpdate: (log: LogEntry[]) => void;
}

export type LogEntry =
	| {
			id: string;
			kind: "line";
			nodeId: string;
			speaker?: string;
			text: string;
			ai?: boolean;
			// 이전 대화 기록을 재생해서 보여준 줄이면 true (재생성/스와이프/줄번호 카운트 대상 아님).
			historical?: boolean;
			// 이 줄이 표시된 시점의 게임 상태 스냅샷. 있으면 이 지점에서 분기(포크)할 수 있습니다.
			state?: GameState;
	  }
	| { id: string; kind: "choice"; nodeId: string; text: string };

export type GameStageDisplay =
	| {
			kind: "line";
			speaker?: string;
			text: string;
			ai?: boolean;
			historical?: boolean;
	  }
	| {
			kind: "choice";
			choices: { text: string }[];
	  };

export class GameStageController {
	private readonly options: GameStageControllerOptions;

	// 캐릭터별 conversation을 세션 동안 들고 있는 트래커 (캐릭터당 conversation 1개)
	private readonly npcTracker: NpcConversationTracker;

	private app?: PIXI.Application;
	private host?: GameHost;

	private resolveLine: (() => void) | null = null;
	private resolveChoice: ((index: number) => void) | null = null;

	// iframe이 보고해준 "지금 플레이어 위치" 캐시. 수동 저장 버튼을 누르면
	// 이 값을 기준으로 실제 DB에 씁니다.
	private lastProgress: { nodeId: string; state: GameState } | null = $state(null);

	// 세션 동안 쌓이는 플레이 로그 (백로그 패널용). 앱을 껐다 켜면 사라집니다.
	private log: LogEntry[] = $state([]);

	// 지금 AI 노드 하나를 처리하는 중이면 그 nodeId가 들어있습니다. 같은 AI 노드
	// 안에서 여러 줄/여러 턴이 오가는 동안 계속 "AI가 만든 내용이다"로 표시하다가,
	// 다른 노드로 넘어가는 textbox.show/choiceBox.show가 오면 자동으로 풀립니다.
	private currentAiNodeId: string | null = null;

	// handleChoiceShow가 세팅해두고, selectChoice가 소비합니다 (고른 선택지 텍스트를 로그에 남기려고).
	private pendingChoiceContext: { nodeId: string; choices: { text: string }[] } | null = null;

	// 지금 라이브로 대기 중인 줄의 내용을 따로 캐싱해둡니다. "이전"으로 과거를
	// 보다가 다시 "다음"으로 끝까지 오면 이걸로 라이브 화면을 복원해요.
	private liveDisplay: {
		speaker?: string;
		text: string;
		ai?: boolean;
		historical?: boolean;
		nodeId: string;
	} | null = $state(null);

	// null이면 라이브. 아니면 lineEntries() 안에서의 인덱스(과거를 스크럽 중).
	private scrubIndex: number | null = $state(null);

	// 재생성/스와이프 전환이 쓰는, 지금 라이브로 떠있는 AI 메시지 row(스와이프 포함).
	private currentMessage: Message | null = $state(null);

	// 지금 몇 번째 줄(1부터)을 보고 있는지. 재생성/스와이프 시 "같은 위치의 줄"을
	// 새 메시지에서 골라 보여주는 데 씁니다 (몇 번째 줄이든 재생성 가능해요).
	private currentTurnLineIndex = $state(0);

	// 재생성 시 같은 조건으로 다시 물어보기 위한 원래 호출 인자.
	private currentAiCallArgs: {
		nodeId: string;
		characterId: string;
		userMessage: string;
		state: GameState;
	} | null = null;

	private destroyed = false;
	private logIdCounter = 0;

	constructor(options: GameStageControllerOptions) {
		this.options = options;
		this.npcTracker = new NpcConversationTracker({
			gameId: options.gameId,
			slotNumber: options.slotNumber,
		});
	}

	async start(): Promise<void> {
		if (this.destroyed) return;

		const scenePromise = this.loadScene();

		await this.initPixi();

		if (this.destroyed) return;

		this.createHost(scenePromise);
	}

	async destroy(): Promise<void> {
		if (this.destroyed) return;

		this.destroyed = true;

		this.resolveLine?.();
		this.resolveChoice?.(-1);

		this.resolveLine = null;
		this.resolveChoice = null;

		this.host?.destroy();
		this.host = undefined;

		this.app?.destroy(true, {
			children: true,
		});
		this.app = undefined;
	}

	advanceLine(): void {
		this.options.onDisplay(null);

		this.liveDisplay = null;
		this.scrubIndex = null;

		this.resolveLine?.();
		this.resolveLine = null;
	}

	/**
	 * 대사창 클릭이든 툴바의 "다음" 버튼이든 여기로 통일해서 들어옵니다.
	 * 과거를 스크럽 중이면 스크럽을 한 칸 진행시키고, 라이브면 실제로 진행시킵니다.
	 * (스크럽 중에 클릭했다고 실제 게임 진행용 Promise를 잘못 resolve하면 안 되니 이렇게 나눴어요.)
	 */
	next(): void {
		if (this.scrubIndex === null) {
			this.advanceLine();
			return;
		}

		const entries = this.lineEntries();
		if (this.scrubIndex < entries.length - 1) {
			this.scrubIndex += 1;
			this.renderScrub();
		} else {
			this.exitScrub();
		}
	}

	/** 과거로 한 칸 이동합니다. 라이브 상태에서 처음 누르면 스크럽 모드로 들어갑니다. */
	previous(): void {
		const entries = this.lineEntries();
		if (entries.length === 0) return;

		if (this.scrubIndex === null) {
			this.scrubIndex = Math.max(0, entries.length - 2);
		} else {
			this.scrubIndex = Math.max(0, this.scrubIndex - 1);
		}

		this.renderScrub();
	}

	isScrubbing(): boolean {
		return this.scrubIndex !== null;
	}

	canGoPrevious(): boolean {
		const entries = this.lineEntries();
		return this.scrubIndex === null ? entries.length > 1 : this.scrubIndex > 0;
	}

	/** 지금 보이는 줄이 AI가 만든 거고, 과거를 보는 중이 아니면 재생성할 수 있어요. */
	canRegenerate(): boolean {
		return (
			this.scrubIndex === null &&
			this.liveDisplay?.ai === true &&
			!this.liveDisplay?.historical
		);
	}

	/** 지금 보고 있는 지점(라이브든 스크럽 중이든)에 state 스냅샷이 있으면 분기할 수 있어요. */
	canFork(): boolean {
		return !!this.currentViewEntry()?.state;
	}

	/**
	 * 지금 보이는 줄의 문구만 다시 굴립니다. 같은 조건(캐릭터/입력/상태)으로 다시
	 * 물어봐서 첫 줄만 교체해요 — 이미 반영된 상태변화/선택지는 그대로 유지됩니다.
	 */
	async regenerate(): Promise<void> {
		if (!this.canRegenerate() || !this.currentAiCallArgs || !this.currentMessage || !this.liveDisplay) return;

		this.setDebug("재생성 중...");

		const { message } = await addSwipe(
			this.currentMessage,
			this.options.npcResponder,
			this.currentAiCallArgs.nodeId,
			{
				characterId: this.currentAiCallArgs.characterId,
				userMessage: this.currentAiCallArgs.userMessage,
				extraContext: this.summarizeState(this.currentAiCallArgs.state),
			},
		);

		this.currentMessage = message;
		this.applyMessageToLiveDisplay(message);

		this.setDebug("재생성 완료");
	}

	/** 지금 스와이프가 몇 번째/총 몇 개인지. 스와이프가 1개뿐이면(=아직 재생성 안 함) null. */
	swipeInfo(): { index: number; total: number } | null {
		// 스크럽 중이거나, 지금 보이는 게 AI가 만든 줄이 아니면(=일반 line 노드)
		// 이전 캐릭터 대화에서 남아있던 currentMessage를 잘못 참조하면 안 돼요.
		if (this.scrubIndex !== null || this.liveDisplay?.ai !== true) return null;

		const swipes = this.currentMessage?.meta?.swipes;
		if (!swipes || swipes.length <= 1) return null;

		return { index: this.currentMessage?.meta?.swipeId ?? 0, total: swipes.length };
	}

	canSwipe(direction: -1 | 1): boolean {
		if (this.scrubIndex !== null) return false;
		const info = this.swipeInfo();
		if (!info) return false;
		return direction === -1 ? info.index > 0 : info.index < info.total - 1;
	}

	/** AI 호출 없이, 이미 만들어둔 스와이프 중 옆 걸로 전환합니다. */
	async goToSwipe(direction: -1 | 1): Promise<void> {
		if (!this.canSwipe(direction) || !this.currentMessage) return;

		const info = this.swipeInfo();
		if (!info) return;

		const message = await selectSwipe(this.currentMessage, info.index + direction);
		this.currentMessage = message;
		this.applyMessageToLiveDisplay(message);
	}

	private applyMessageToLiveDisplay(message: Message): void {
		if (!this.liveDisplay) return;

		// message.content는 그 턴의 모든 줄이 \n으로 합쳐진 것이라, 지금 보고
		// 있던 줄과 같은 위치(0-based)의 줄을 뽑습니다. 새 응답 줄 수가 더 적으면
		// 마지막 줄로 대체돼요.
		const newLines = message.content.split("\n");
		const targetIndex = Math.min(Math.max(this.currentTurnLineIndex - 1, 0), newLines.length - 1);
		const shownText = newLines[targetIndex] ?? message.content;

		this.liveDisplay = { ...this.liveDisplay, text: shownText };
		this.replaceLastLineLogText(shownText);

		this.options.onDisplay({
			kind: "line",
			speaker: this.liveDisplay.speaker,
			text: shownText,
			ai: true,
		});
	}

	/** 지금 보고 있는 지점(라이브든 과거 스크럽 중이든)에서 다른 슬롯으로 분기합니다. */
	async forkFromCurrentView(): Promise<{ ok: boolean; slotNumber?: number; error?: string }> {
		const entry = this.currentViewEntry();
		if (!entry?.state) {
			return { ok: false, error: "분기할 수 없는 지점이에요." };
		}

		const saves = await this.listSaveSlots();
		const usedSlots = new Set(saves.map((s) => s.slotNumber));
		let target = 1;
		while (usedSlots.has(target)) target += 1;

		const result = await this.forkToSlot(entry, target);
		return { ...result, slotNumber: target };
	}

	selectChoice(index: number): void {
		const ctx = this.pendingChoiceContext;
		this.pendingChoiceContext = null;

		const chosenText = ctx?.choices[index]?.text;
		if (ctx && chosenText) {
			this.pushLog({ kind: "choice", nodeId: ctx.nodeId, text: chosenText });
		}

		this.options.onDisplay(null);

		this.resolveChoice?.(index);
		this.resolveChoice = null;
	}

	getLog(): LogEntry[] {
		return this.log;
	}

	/**
	 * 백로그의 특정 지점(줄)에서 다른 슬롯으로 분기합니다. 대화 기록은 슬롯마다
	 * 자연히 분리되니(game_slot_conversations가 slot_number로 구분) 여기서
	 * 따로 복사/차단 처리를 안 해도 새 슬롯은 AI와 새로 대화를 시작하게 됩니다.
	 */
	async forkToSlot(entry: LogEntry, targetSlotNumber: number): Promise<{ ok: boolean; error?: string }> {
		if (entry.kind !== "line" || !entry.state) {
			return { ok: false, error: "이 항목에서는 분기할 수 없어요." };
		}

		// 분기 지점까지의 대사/선택 기록을 그대로 실어서, 새 슬롯에서도
		// "그 전에 무슨 일이 있었는지" 백로그로 볼 수 있게 합니다.
		const idx = this.log.findIndex((e) => e.id === entry.id);
		const history = idx >= 0 ? this.log.slice(0, idx + 1) : this.log;

		await gameCommands.saveGame({
			gameId: this.options.gameId,
			input: {
				slotNumber: targetSlotNumber,
				currentNodeId: entry.nodeId,
				stateSnapshot: { vars: entry.state, history },
			},
		});

		return { ok: true };
	}

	/** 백로그에서 분기할 슬롯을 고를 때, 빈 슬롯/쓰던 슬롯을 보여주는 용도입니다. */
	async listSaveSlots() {
		return gameCommands.listSaves({ gameId: this.options.gameId });
	}

	/** 플레이어가 저장 버튼을 눌렀을 때 호출합니다. 실제로 DB에 쓰는 유일한 지점입니다. */
	async manualSave(): Promise<{ ok: boolean }> {
		if (!this.lastProgress) {
			this.setDebug("아직 저장할 진행 상황이 없어요.");
			return { ok: false };
		}

		await this.saveGame(this.lastProgress.nodeId, this.lastProgress.state);
		this.setDebug("저장했어요.");
		return { ok: true };
	}

	/** 현재 저장 가능한 진행 상황이 있는지 (저장 버튼 활성화 여부 등에 씁니다) */
	hasProgress(): boolean {
		return this.lastProgress !== null;
	}

	private async initPixi(): Promise<void> {
		this.setDebug("Pixi 초기화 중...");

		const app = new PIXI.Application();

		await app.init({
			width: 960,
			height: 540,
			background: 0x1e1e24,
			antialias: true,
		});

		if (this.destroyed) {
			app.destroy(true);
			return;
		}

		this.app = app;

		this.options.canvasHost.appendChild(app.canvas);

		const placeholder = new PIXI.Graphics().rect(0, 0, 960, 540).fill(0x33333d);

		app.stage.addChild(placeholder);
	}

	private async loadScene() {
		this.setDebug("게임 데이터 로딩 중...");

		const bundle = await gameCommands.loadGameBundle({
			gameId: this.options.gameId,
		});

		const sceneRow = findScene(bundle, this.options.startSceneKey);

		if (!sceneRow) {
			throw new Error(`씬을 찾을 수 없습니다: ${this.options.startSceneKey}`);
		}

		return {
			scene: buildScene(sceneRow),
			stateDefs: buildStateDefs(bundle),
		};
	}

	private createHost(scenePromise: ReturnType<typeof this.loadScene>): void {
		this.host = new GameHost(this.options.iframeEl, {
			onTextboxShow: (args) => {
				return this.handleTextboxShow(args);
			},

			onChoiceBoxShow: (args) => {
				return this.handleChoiceShow(args);
			},

			onProgressReport: async ({ nodeId, state }) => {
				this.lastProgress = { nodeId, state };
			},

			onStorageLoad: async () => {
				return this.loadGame();
			},

			onNpcSay: async ({ nodeId, characterId, userMessage, state }) => {
				return this.handleNpcSay(nodeId, characterId, userMessage, state);
			},

			onReady: () => {
				this.handleReady(scenePromise);
			},
		});
	}

	private handleTextboxShow(args: {
		nodeId: string;
		speaker?: string;
		text: string;
		historical?: boolean;
	}): Promise<void> {
		this.setDebug("textbox.show 수신");

		const isAi = this.currentAiNodeId === args.nodeId;
		if (!isAi) {
			this.currentAiNodeId = null; // 다른 노드로 넘어왔으니 AI 표시를 풉니다.
			this.currentMessage = null; // 스와이프 정보도 같이 비워서 안 섞이게 합니다.
		}
		// 이전 대화 리플레이 줄은 "새 턴의 몇 번째 줄"이 아니니 카운트에서 뺍니다
		// (재생성/스와이프의 위치 매칭이 어긋나지 않게 하려고요).
		if (isAi && !args.historical) this.currentTurnLineIndex += 1;

		// progress.report가 항상 이 노드로 진입하기 직전에 먼저 도착하므로,
		// 지금 lastProgress가 곧 "이 줄이 표시되는 시점의 상태"입니다.
		const stateSnapshot =
			this.lastProgress?.nodeId === args.nodeId ? this.lastProgress.state : undefined;

		this.pushLog({
			kind: "line",
			nodeId: args.nodeId,
			speaker: args.speaker,
			text: args.text,
			ai: isAi || undefined,
			historical: args.historical,
			state: stateSnapshot,
		});

		this.liveDisplay = {
			speaker: args.speaker,
			text: args.text,
			ai: isAi || undefined,
			historical: args.historical,
			nodeId: args.nodeId,
		};
		this.scrubIndex = null;

		return new Promise<void>((resolve) => {
			this.resolveLine = resolve;

			this.options.onDisplay({
				kind: "line",
				speaker: args.speaker,
				text: args.text,
				ai: isAi || undefined,
				historical: args.historical,
			});
		});
	}

	private handleChoiceShow(args: {
		nodeId: string;
		choices: { text: string }[];
	}): Promise<number> {
		this.setDebug("choiceBox.show 수신");

		if (this.currentAiNodeId !== args.nodeId) this.currentAiNodeId = null;

		this.pendingChoiceContext = { nodeId: args.nodeId, choices: args.choices };

		return new Promise<number>((resolve) => {
			this.resolveChoice = resolve;

			this.options.onDisplay({
				kind: "choice",
				choices: args.choices,
			});
		});
	}

	private async saveGame(nodeId: string, state: GameState): Promise<void> {
		await gameCommands.saveGame({
			gameId: this.options.gameId,

			input: {
				slotNumber: this.options.slotNumber,
				currentNodeId: nodeId,
				stateSnapshot: { vars: state, history: this.log },
			},
		});
	}

	private async loadGame() {
		const save = await gameCommands.loadSave({
			gameId: this.options.gameId,
			slotNumber: this.options.slotNumber,
		});

		if (!save || !save.currentNodeId) {
			return null;
		}

		const data = asSaveData(save.stateSnapshot);

		// 이어하기 시점에 백로그를 복원해둡니다. 이 뒤로 진행하면서 새 항목이 계속 쌓여요.
		this.log = data.history as LogEntry[];
		this.options.onLogUpdate(this.log);

		return {
			nodeId: save.currentNodeId,
			state: data.vars,
		};
	}

	private async handleNpcSay(
		nodeId: string,
		characterId: string,
		userMessage: string,
		state: GameState,
	): Promise<{
		speakerName: string;
		lines: string[];
		stateChanges: StateChangeInstruction[];
		choices: string[];
		history: { speaker: string; text: string }[];
	}> {
		this.setDebug("npc.say 수신");

		// 뒤이어 오는 textbox.show/choiceBox.show가 "이건 AI 노드 안이다"라고
		// 알 수 있게 표시해둡니다. (여러 줄/여러 턴 내내 유지됩니다)
		this.currentAiNodeId = nodeId;
		this.currentAiCallArgs = { nodeId, characterId, userMessage, state };
		this.currentTurnLineIndex = 0; // 새 턴 시작이니 "몇 번째 줄인지" 다시 셉니다.

		const { speakerName, parsed, message, history } = await callNpcStructured(
			this.options.npcResponder,
			this.npcTracker,
			nodeId,
			{
				characterId,
				userMessage,
				extraContext: this.summarizeState(state),
			},
		);

		this.currentMessage = message;

		return {
			speakerName,
			lines: parsed.lines,
			stateChanges: parsed.stateChanges,
			choices: parsed.choices,
			history,
		};
	}

	private async handleReady(scenePromise: ReturnType<typeof this.loadScene>): Promise<void> {
		try {
			const { scene, stateDefs } = await scenePromise;

			if (this.destroyed) return;

			this.setDebug("GAME_INIT 전송");

			this.host?.sendInit({
				scene,
				stateDefs,
			});
		} catch (error) {
			this.setDebug(`게임 데이터 로딩 실패: ${String(error)}`);

			console.error(error);
		}
	}

	private summarizeState(state: GameState): string {
		return Object.entries(state)
			.map(([key, value]) => `${key}: ${value}`)
			.join(", ");
	}

	private lineEntries(): (LogEntry & { kind: "line" })[] {
		return this.log.filter((e): e is LogEntry & { kind: "line" } => e.kind === "line");
	}

	private renderScrub(): void {
		const entries = this.lineEntries();
		const entry = this.scrubIndex !== null ? entries[this.scrubIndex] : undefined;
		if (!entry) return;

		this.options.onDisplay({
			kind: "line",
			speaker: entry.speaker,
			text: entry.text,
			ai: entry.ai,
		});
	}

	private exitScrub(): void {
		this.scrubIndex = null;

		if (this.liveDisplay) {
			this.options.onDisplay({
				kind: "line",
				speaker: this.liveDisplay.speaker,
				text: this.liveDisplay.text,
				ai: this.liveDisplay.ai,
			});
		}
	}

	/** 라이브 상태면 마지막 줄, 스크럽 중이면 지금 보고 있는 그 줄을 돌려줍니다. */
	private currentViewEntry(): (LogEntry & { kind: "line" }) | undefined {
		const entries = this.lineEntries();
		return this.scrubIndex !== null ? entries[this.scrubIndex] : entries[entries.length - 1];
	}

	private replaceLastLineLogText(newText: string): void {
		const lastIndex = this.log.length - 1;
		const last = this.log[lastIndex];
		if (!last || last.kind !== "line") return;

		this.log = [...this.log.slice(0, lastIndex), { ...last, text: newText }];
		this.options.onLogUpdate(this.log);
	}

	private pushLog(entry: Omit<LogEntry, "id">): void {
		this.log = [...this.log, { ...entry, id: this.nextLogId() } as LogEntry];
		this.options.onLogUpdate(this.log);
	}

	private nextLogId(): string {
		return `log_${++this.logIdCounter}`;
	}

	private setDebug(message: string): void {
		this.options.onDebug(message);
	}
}