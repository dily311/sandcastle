// CSP에 unsafe-eval이 없어서 Pixi의 셰이더 코드 생성을 eval 없는 구현으로 바꾼다 (tauri.conf.json의 csp)
import "pixi.js/unsafe-eval";
import { Application } from "pixi.js";
import { commands, type MascotChatHistoryItem, type WidgetMascotNotifyPayload, type WidgetMascotSetting_Serialize } from "$ipc/bindings";
import { onEvent } from "$shared/ipc/events";
import { pixiInteractive } from "$/lib/interaction/interactive-pixi";
import { handleContextMenuForMascot } from "$/lib/contextmenu/contextMenu.svelte";
import { CharacterManager } from "./CharaterManager";
import { PeerChatManager } from "../PeerChatManager";
import { WIDGET_MASCOT_FEATURE_ID } from "../mascotFeature.svelte";
import type { Character } from "../character/Character";
import {
	defaultWidgetMascotSetting,
	isLongAbsence,
	pickDialogueLine,
	type MascotDialogueKey,
	type MascotOverlayState,
} from "../simejiType";

interface BubbleEntry {
	text: string;
	speaker: string | null;
	mood: MascotOverlayState["bubbleMood"];
	expiresAt: number;
}

export class WidgetGame {
	app: Application;
	characters = new CharacterManager();
	private peerChat!: PeerChatManager;
	private setting: WidgetMascotSetting_Serialize = defaultWidgetMascotSetting();
	private running = false;
	private destroyed = false;

	private interactiveStops = new Map<string, () => void>();
	private bubbles = new Map<string, BubbleEntry>();
	private chatHistories = new Map<string, MascotChatHistoryItem[]>();
	private charAvatarMap = new Map<string, string | null>();
	private userName = "유저";

	private unlistenSetting?: () => void;
	private unlistenNotify?: () => void;
	private onOverlayUpdate?: (items: MascotOverlayState[]) => void;
	private lastOverlayPush = 0;

	constructor(onOverlayUpdate?: (items: MascotOverlayState[]) => void) {
		this.app = new Application();
		this.onOverlayUpdate = onOverlayUpdate;
		this.peerChat = new PeerChatManager(this.characters, {
			getSetting: () => this.setting,
			getUserName: () => this.userName,
			showBubble: (mascotId, text, durationMs, mood, speaker) => {
				this.showBubble(mascotId, text, durationMs, mood, speaker ?? null);
			},
		});
	}

	async init(container: HTMLElement) {
		await this.app.init({
			resizeTo: container,
			backgroundAlpha: 0,
			sharedTicker: false,
			autoStart: true,
		});

		if (this.destroyed) {
			this.app.destroy({ removeView: true });
			return;
		}

		container.appendChild(this.app.canvas);

		await this.loadReferenceMetadata();
		const loaded = await commands.getWidgetMascotSettings().catch(() => defaultWidgetMascotSetting());
		await this.syncSetting(loaded, true);

		this.unlistenSetting = await onEvent<WidgetMascotSetting_Serialize>("widget_mascot::changed", (next) => {
			void this.syncSetting(next, false);
		});

		this.unlistenNotify = await onEvent<WidgetMascotNotifyPayload>("widget_mascot::notify", (payload) => {
			this.handleNotification(payload);
		});

		window.addEventListener("resize", this.handleResize);
		this.app.ticker.add(this.update);
		this.running = true;
	}

	private async loadReferenceMetadata() {
		try {
			const [chars, personas, appSet] = await Promise.all([
				commands.getAllCharacters().catch(() => []),
				commands.getAllPersonas().catch(() => []),
				commands.getAppSetting().catch(() => null),
			]);
			this.charAvatarMap.clear();
			for (const c of chars) this.charAvatarMap.set(c.id, c.avatar ?? null);

			const defaultPid = appSet?.selectedPersonaId;
			const persona = personas.find((p) => p.id === defaultPid) ?? personas[0];
			if (persona?.name) this.userName = persona.name;
		} catch {
			// 메타데이터 로드 실패해도 기본값으로 동작
		}
	}

	private handleResize = () => {
		if (this.destroyed) return;
		this.characters.updateFloor(this.app.screen.height);
	};

	async syncSetting(next: WidgetMascotSetting_Serialize, isInitial: boolean) {
		if (this.destroyed) return;
		this.setting = next;

		const enabledConfigs = next.characters.filter((c) => c.enabled);
		const enabledIds = new Set(enabledConfigs.map((c) => c.id));

		// 꺼지거나 삭제된 시메지 제거
		for (const existing of this.characters.getAll()) {
			if (!enabledIds.has(existing.id)) {
				this.removeOne(existing.id);
			}
		}

		const newlySpawned: { char: Character; wasLongAbsence: boolean }[] = [];
		const floorY = this.app.screen.height || window.innerHeight;
		const screenW = this.app.screen.width || window.innerWidth;

		for (let idx = 0; idx < enabledConfigs.length; idx++) {
			const cfg = enabledConfigs[idx];
			const avatar = cfg.characterId ? (this.charAvatarMap.get(cfg.characterId) ?? null) : null;
			const existing = this.characters.get(cfg.id);

			if (existing) {
				await existing.applyConfig(cfg, avatar);
			} else {
				const spreadX = enabledConfigs.length === 1
					? screenW / 2
					: Math.max(100, Math.min(screenW - 100, (screenW / (enabledConfigs.length + 1)) * (idx + 1)));

				const char = await this.characters.spawnCharacter(cfg, spreadX, floorY, avatar, {
					onDialogueTrigger: (ch, key, mood) => this.speakFromConfig(ch, key, mood ?? "normal"),
					onRightClick: (e, ch) => this.handleRightClick(e, ch),
				});

				if (this.destroyed) {
					char.destroy();
					return;
				}

				this.app.stage.addChild(char.container);
				const stop = pixiInteractive(this.app, char.hitbox, `widget.mascot.${char.id}`);
				this.interactiveStops.set(char.id, stop);

				newlySpawned.push({ char, wasLongAbsence: isLongAbsence(cfg.lastSeenAt) });
			}
		}

		// 새로 등장한 시메지 인사 / 7일 이상 오랜만 대사 출력 + lastSeenAt 갱신
		if (newlySpawned.length > 0) {
			for (const { char, wasLongAbsence } of newlySpawned) {
				const key: MascotDialogueKey = wasLongAbsence ? "longAbsence" : "greeting";
				this.speakFromConfig(char, key, wasLongAbsence ? "happy" : "normal");
				if (wasLongAbsence) {
					char.showMood("happy", 3600);
				}
			}

			if (isInitial) {
				const now = Date.now();
				const updated: WidgetMascotSetting_Serialize = {
					...this.setting,
					characters: this.setting.characters.map((c) => (enabledIds.has(c.id) ? { ...c, lastSeenAt: now } : c)),
				};
				this.setting = updated;
				void commands.saveWidgetMascotSettings(updated).catch(() => {});
			}
		}

		this.pushOverlay(true);
	}

	private removeOne(id: string) {
		this.interactiveStops.get(id)?.();
		this.interactiveStops.delete(id);
		this.bubbles.delete(id);
		this.chatHistories.delete(id);
		this.characters.removeCharacter(id);
	}

	private handleNotification(payload: WidgetMascotNotifyPayload) {
		if (this.destroyed) return;
		const notifyText = payload.body || payload.targetName || "새 알림";
		for (const char of this.characters.getAll()) {
			const line =
				pickDialogueLine(char.config.dialogues, "notify", {
					user: this.userName,
					char: char.config.name,
					notify: notifyText,
				}) ?? `${notifyText}`;
			char.showMood("working", 3800);
			this.showBubble(char.id, line, 4800, "notify", char.config.name);
		}
	}

	speakFromConfig(
		char: Character,
		key: MascotDialogueKey,
		mood: MascotOverlayState["bubbleMood"] = "normal",
		extraVars: { notify?: string; gift?: string } = {},
	) {
		const line = pickDialogueLine(char.config.dialogues, key, {
			user: this.userName,
			char: char.config.name,
			...extraVars,
		});
		if (!line) return;
		this.showBubble(char.id, line, 4200, mood, char.config.name);
	}

	showBubble(
		mascotId: string,
		text: string,
		durationMs = 4200,
		mood: MascotOverlayState["bubbleMood"] = "normal",
		speaker: string | null = null,
	) {
		this.bubbles.set(mascotId, {
			text,
			speaker,
			mood,
			expiresAt: Date.now() + durationMs,
		});
		this.pushOverlay(true);
	}

	async sendChatToMascot(mascotId: string, userMessage: string): Promise<string | null> {
		const char = this.characters.get(mascotId);
		if (!char) return null;
		const trimmed = userMessage.trim();
		if (!trimmed) return null;

		char.setUserChatting(true);
		this.showBubble(mascotId, "...", 15_000, "normal", char.config.name);

		try {
			let reply: string | null = null;
			if (char.config.useAiDialogue) {
				const history = this.chatHistories.get(mascotId) ?? [];
				try {
					reply = await commands.mascotGenerateReply({
						mascotId,
						userMessage: trimmed,
						eventKind: "chat",
						history,
					});
					if (reply) {
						history.push({ role: "user", content: trimmed }, { role: "assistant", content: reply });
						this.chatHistories.set(mascotId, history.slice(-12));
					}
				} catch (e) {
					console.info("[Mascot] AI 응답 실패 → 설정 대사로 폴백", e);
				}
			}
			if (!reply) {
				reply =
					pickDialogueLine(char.config.dialogues, "happy", { user: this.userName, char: char.config.name }) ??
					pickDialogueLine(char.config.dialogues, "idleChatter", { user: this.userName, char: char.config.name }) ??
					"응, 듣고 있어!";
			}
			char.showMood("happy", 3400);
			this.showBubble(mascotId, reply, 5600, "happy", char.config.name);
			return reply;
		} finally {
			char.setUserChatting(false);
		}
	}

	async giveGiftToMascot(mascotId: string, giftName: string) {
		const char = this.characters.get(mascotId);
		if (!char) return;
		const gift = giftName.trim() || "선물";
		char.showMood("happy", 4200);

		if (char.config.useAiDialogue) {
			this.showBubble(mascotId, `🎁 ${gift}...!`, 8000, "happy", char.config.name);
			try {
				const reply = await commands.mascotGenerateReply({
					mascotId,
					userMessage: `${this.userName}이(가) ${gift}을(를) 선물로 주었다.`,
					eventKind: "gift",
					history: [],
				});
				if (reply) {
					this.showBubble(mascotId, reply, 5200, "happy", char.config.name);
					return;
				}
			} catch {
				// 폴백 대사 사용
			}
		}

		const line =
			pickDialogueLine(char.config.dialogues, "gift", {
				user: this.userName,
				char: char.config.name,
				gift,
			}) ?? `와, ${gift} 고마워!`;
		this.showBubble(mascotId, line, 4800, "happy", char.config.name);
	}

	petMascot(mascotId: string) {
		const char = this.characters.get(mascotId);
		if (!char) return;
		char.showMood("happy", 3600);
		this.speakFromConfig(char, "happy", "happy");
	}

	async triggerPeerChat(mascotId: string) {
		const started = await this.peerChat.triggerManual(mascotId);
		if (!started) {
			const char = this.characters.get(mascotId);
			if (char) {
				this.showBubble(mascotId, "지금은 대화할 다른 시메지가 없거나 바빠!", 3200, "normal", char.config.name);
			}
		}
	}

	async hideMascot(mascotId: string) {
		const updated: WidgetMascotSetting_Serialize = {
			...this.setting,
			characters: this.setting.characters.map((c) => (c.id === mascotId ? { ...c, enabled: false } : c)),
		};
		this.setting = updated;
		this.removeOne(mascotId);
		this.pushOverlay(true);
		await commands.saveWidgetMascotSettings(updated).catch(() => {});
	}

	getCharacterCount(): number {
		return this.characters.count();
	}

	private handleRightClick(event: unknown, char: Character) {
		const global = (event as { global?: { x: number; y: number } })?.global ?? { x: char.container.x, y: char.container.y - 80 };
		handleContextMenuForMascot(global, WIDGET_MASCOT_FEATURE_ID, {
			data: { char, game: this, mascotCount: this.characters.count() },
		});
	}

	private update = () => {
		if (!this.running || this.destroyed) return;
		const deltaMS = this.app.ticker.deltaMS;
		this.characters.update(deltaMS);
		this.peerChat.update(deltaMS);
		this.pushOverlay(false);
	};

	private pushOverlay(force: boolean) {
		if (!this.onOverlayUpdate) return;
		const now = performance.now();
		if (!force && now - this.lastOverlayPush < 50) return;
		this.lastOverlayPush = now;

		const wallNow = Date.now();
		const items: MascotOverlayState[] = [];
		for (const char of this.characters.getAll()) {
			const bubble = this.bubbles.get(char.id);
			const activeBubble = bubble && bubble.expiresAt > wallNow ? bubble : null;
			if (bubble && !activeBubble) {
				this.bubbles.delete(char.id);
			}
			items.push({
				id: char.id,
				name: char.config.name,
				x: char.container.x,
				topY: Math.max(24, char.container.y - char.getDisplayHeight() - 10),
				bubbleText: activeBubble?.text ?? null,
				bubbleSpeaker: activeBubble?.speaker ?? null,
				bubbleMood: activeBubble?.mood ?? "normal",
			});
		}
		this.onOverlayUpdate(items);
	}

	pause() {
		this.running = false;
		this.app.stop();
		for (const stop of this.interactiveStops.values()) stop();
		this.interactiveStops.clear();
	}

	start() {
		this.running = true;
		this.app.ticker.start();
		this.app.start();
		for (const char of this.characters.getAll()) {
			if (!this.interactiveStops.has(char.id)) {
				this.interactiveStops.set(char.id, pixiInteractive(this.app, char.hitbox, `widget.mascot.${char.id}`));
			}
		}
	}

	destroy() {
		this.destroyed = true;
		this.running = false;
		window.removeEventListener("resize", this.handleResize);
		this.unlistenSetting?.();
		this.unlistenNotify?.();
		this.peerChat.destroy();
		for (const stop of this.interactiveStops.values()) stop();
		this.interactiveStops.clear();
		this.characters.clear();
		try {
			this.app.ticker.stop();
			this.app.destroy({ removeView: true });
		} catch {
			// 이미 정리됨
		}
	}
}
