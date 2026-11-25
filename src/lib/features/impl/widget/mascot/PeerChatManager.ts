import { commands, type MascotPeerChatTurn, type WidgetMascotSetting_Serialize } from "$ipc/bindings";
import type { Character } from "./character/Character";
import type { CharacterManager } from "./pixi/CharaterManager";
import { pickDialogueLine } from "./simejiType";

export interface PeerChatHooks {
	getSetting: () => WidgetMascotSetting_Serialize;
	getUserName: () => string;
	showBubble: (mascotId: string, text: string, durationMs: number, mood: "peer", speaker?: string) => void;
}

export class PeerChatManager {
	private manager: CharacterManager;
	private hooks: PeerChatHooks;
	private elapsedMs = 0;
	private activeConversation = false;
	private disposed = false;

	constructor(manager: CharacterManager, hooks: PeerChatHooks) {
		this.manager = manager;
		this.hooks = hooks;
	}

	resetTimer() {
		this.elapsedMs = 0;
	}

	update(deltaMS: number) {
		if (this.disposed || this.activeConversation) return;
		const setting = this.hooks.getSetting();
		if (!setting.pairChatEnabled) return;
		if (this.manager.count() < 2) return;

		this.elapsedMs += deltaMS;
		const intervalMs = Math.max(15_000, (setting.pairChatIntervalSec || 90) * 1000);
		if (this.elapsedMs < intervalMs) return;

		this.elapsedMs = 0;
		const candidates = this.manager.getAll().filter((c) => !c.isBusy() && c.behaviorMode === "automatic");
		if (candidates.length < 2) return;

		// 무작위 쌍 선택 (가장 가까운 쌍 우선 확률 높임)
		const first = candidates[Math.floor(Math.random() * candidates.length)];
		const others = candidates.filter((c) => c.id !== first.id);
		others.sort((a, b) => Math.abs(a.container.x - first.container.x) - Math.abs(b.container.x - first.container.x));
		const second = others[0];
		void this.startConversation(first, second, true);
	}

	async triggerManual(mascotAId: string, mascotBId?: string): Promise<boolean> {
		if (this.disposed || this.activeConversation) return false;
		const a = this.manager.get(mascotAId);
		if (!a) return false;
		const b = mascotBId ? this.manager.get(mascotBId) : this.manager.findNearestOther(mascotAId);
		if (!b || b.id === a.id) return false;

		this.elapsedMs = 0;
		await this.startConversation(a, b, false);
		return true;
	}

	/** auto: 타이머가 시작한 대화 (AI를 부르면 `run_by = auto`로 기록됨) */
	private async startConversation(a: Character, b: Character, auto: boolean) {
		if (this.activeConversation || this.disposed) return;
		this.activeConversation = true;
		a.setPeerChatting(true);
		b.setPeerChatting(true);

		try {
			const setting = this.hooks.getSetting();
			const targetGap = Math.max(110, Math.min(260, (setting.pairChatDistancePx || 260) * 0.65));
			const midX = Math.max(120, Math.min(window.innerWidth - 120, (a.container.x + b.container.x) / 2));
			const leftChar = a.container.x <= b.container.x ? a : b;
			const rightChar = leftChar === a ? b : a;
			const leftTarget = Math.max(70, midX - targetGap / 2);
			const rightTarget = Math.min(window.innerWidth - 70, midX + targetGap / 2);

			// 두 시메지가 서로 가까이 걸어가기 + 동시에 대사 준비
			const [turns] = await Promise.all([
				this.fetchTurns(a, b, setting.pairChatUseAi, auto),
				Promise.all([this.walkAndWait(leftChar, leftTarget, 5500), this.walkAndWait(rightChar, rightTarget, 5500)]),
			]);

			if (this.disposed) return;

			// 서로 마주보기
			leftChar.faceToward(rightChar.container.x);
			rightChar.faceToward(leftChar.container.x);

			// 대사 턴 순차 재생
			for (const turn of turns) {
				if (this.disposed) break;
				const speaker = this.manager.get(turn.mascotId);
				if (!speaker) continue;
				const listener = speaker.id === a.id ? b : a;
				speaker.faceToward(listener.container.x);
				listener.faceToward(speaker.container.x);
				speaker.animations.play("working");
				listener.animations.play("idle");
				this.hooks.showBubble(speaker.id, turn.text, 3500, "peer", speaker.config.name);
				await this.sleep(3700);
			}
		} catch (e) {
			console.warn("[Mascot] 시메지 간 대화 중 오류", e);
		} finally {
			a.setPeerChatting(false);
			b.setPeerChatting(false);
			if (!this.disposed) {
				if (a.behaviorMode === "automatic") a.animations.play("idle");
				if (b.behaviorMode === "automatic") b.animations.play("idle");
			}
			this.activeConversation = false;
		}
	}

	private async fetchTurns(a: Character, b: Character, useAi: boolean, auto: boolean): Promise<MascotPeerChatTurn[]> {
		if (useAi) {
			try {
				const turns = await commands.mascotGeneratePeerChat({
					mascotAId: a.id,
					mascotBId: b.id,
					topic: null,
					auto,
				});
				if (turns && turns.length > 0) return turns;
			} catch (e) {
				console.info("[Mascot] AI 시메지 간 대화 실패 → 기본 대사로 폴백", e);
			}
		}

		// 오프라인 / AI 미연결 폴백 대사
		const userName = this.hooks.getUserName();
		const lineA =
			pickDialogueLine(a.config.dialogues, "peerChat", { user: userName, char: a.config.name }) ??
			pickDialogueLine(a.config.dialogues, "greeting", { user: userName, char: a.config.name }) ??
			`${b.config.name}, 안녕! 오늘 기분 어때?`;
		const lineB =
			pickDialogueLine(b.config.dialogues, "peerChat", { user: userName, char: b.config.name }) ??
			pickDialogueLine(b.config.dialogues, "idleChatter", { user: userName, char: b.config.name }) ??
			`안녕 ${a.config.name}! 화면 위가 참 포근하네~`;

		return [
			{ mascotId: a.id, name: a.config.name, text: lineA },
			{ mascotId: b.id, name: b.config.name, text: lineB },
		];
	}

	private walkAndWait(char: Character, targetX: number, timeoutMs: number): Promise<void> {
		return new Promise((resolve) => {
			if (Math.abs(char.container.x - targetX) < 16) {
				resolve();
				return;
			}
			let done = false;
			const finish = () => {
				if (done) return;
				done = true;
				clearTimeout(timer);
				resolve();
			};
			const timer = setTimeout(finish, timeoutMs);
			char.walkTo(targetX, finish);
		});
	}

	private sleep(ms: number): Promise<void> {
		return new Promise((resolve) => setTimeout(resolve, ms));
	}

	destroy() {
		this.disposed = true;
	}
}
