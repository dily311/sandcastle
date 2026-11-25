import { Container, Graphics } from "pixi.js";
import type { MascotCharacterConfig_Serialize } from "$ipc/bindings";
import { AnimationController } from "./AnimationController";
import { BehaviorController } from "./BehaviorController";
import { buildBehaviorsFromWeights, type BehaviorMode, type BehaviorType, type MascotDialogueKey } from "../simejiType";

export interface CharacterCallbacks {
	onDialogueTrigger?: (char: Character, key: MascotDialogueKey, mood?: "normal" | "happy" | "annoyed") => void;
	onRightClick?: (event: unknown, char: Character) => void;
}

export class Character {
	readonly id: string;
	config: MascotCharacterConfig_Serialize;
	container = new Container();
	hitbox!: Graphics;
	behaviorController!: BehaviorController;
	animations = new AnimationController();
	behaviorMode: BehaviorMode = "automatic";

	speed = 80;
	direction: "left" | "right" = "right";
	currentAction: BehaviorType = "idle";
	targetX: number | null = null;
	floorY = window.innerHeight;

	private callbacks: CharacterCallbacks;
	private dragging = false;
	private peerChatting = false;
	private userChatting = false;
	private moodTimerMs = 0;
	private fallVelocity = 0;
	private onArriveCallback: (() => void) | null = null;

	// 드래그/클릭 빈도로 짜증 감지
	private recentDragTimes: number[] = [];
	private lastDragLineAt = 0;
	private lastClickAt = 0;

	constructor(config: MascotCharacterConfig_Serialize, callbacks: CharacterCallbacks = {}) {
		this.id = config.id;
		this.config = config;
		this.callbacks = callbacks;
		this.speed = Math.max(20, Math.min(300, config.speed || 80));
	}

	async init(fallbackAvatarPath?: string | null) {
		await this.animations.init(this.config.images, this.container, this.config.scale, fallbackAvatarPath);
		const behaviors = buildBehaviorsFromWeights(this.config.behaviorWeights, this.config.actionIntervalSec);
		this.behaviorController = new BehaviorController(this, behaviors, this.config.actionIntervalSec);

		this.hitbox = new Graphics();
		this.rebuildHitbox();
		this.container.addChild(this.hitbox);
		this.hitbox.eventMode = "static";
		this.handleInteractions();
	}

	async applyConfig(next: MascotCharacterConfig_Serialize, fallbackAvatarPath?: string | null) {
		const imagesChanged = JSON.stringify(this.config.images) !== JSON.stringify(next.images);
		const scaleChanged = this.config.scale !== next.scale;
		this.config = next;
		this.speed = Math.max(20, Math.min(300, next.speed || 80));

		if (imagesChanged) {
			await this.animations.init(next.images, this.container, next.scale, fallbackAvatarPath);
			this.rebuildHitbox();
			this.animations.play(this.currentAction);
		} else if (scaleChanged) {
			this.animations.setScale(next.scale);
			this.rebuildHitbox();
		}

		const behaviors = buildBehaviorsFromWeights(next.behaviorWeights, next.actionIntervalSec);
		this.behaviorController.updateConfig(behaviors, next.actionIntervalSec);
	}

	private rebuildHitbox() {
		if (!this.hitbox) return;
		const { width, height } = this.animations.getDisplaySize();
		const w = Math.max(64, width);
		const h = Math.max(64, height);
		this.hitbox.clear();
		this.hitbox.rect(-w / 2, -h, w, h).fill({ color: 0xffffff, alpha: 0.001 });
	}

	getDisplayHeight(): number {
		return this.animations.getDisplaySize().height;
	}

	isBusy(): boolean {
		return this.dragging || this.peerChatting || this.userChatting || this.moodTimerMs > 0;
	}

	setPeerChatting(active: boolean) {
		this.peerChatting = active;
		if (active) {
			this.targetX = null;
			this.onArriveCallback = null;
		}
	}

	setUserChatting(active: boolean) {
		this.userChatting = active;
		if (active) {
			this.targetX = null;
			this.animations.play("working");
		} else if (this.behaviorMode === "automatic") {
			this.animations.play("idle");
		}
	}

	showMood(mood: "happy" | "annoyed" | "working", durationMs = 3200) {
		this.moodTimerMs = Math.max(this.moodTimerMs, durationMs);
		this.targetX = null;
		this.currentAction = mood;
		this.animations.play(mood);
	}

	triggerIdleChatter() {
		this.callbacks.onDialogueTrigger?.(this, "idleChatter", "normal");
	}

	private handleInteractions() {
		let dragOffsetX = 0;
		let dragOffsetY = 0;
		let startX = 0;
		let startY = 0;
		let moved = false;

		this.hitbox.cursor = "grab";

		this.hitbox.on("rightclick", (event) => {
			this.callbacks.onRightClick?.(event, this);
		});

		this.hitbox.on("pointerdown", (event) => {
			if ("button" in event && event.button === 2) return;
			this.dragging = true;
			moved = false;
			this.targetX = null;
			this.onArriveCallback = null;
			this.fallVelocity = 0;
			startX = event.global.x;
			startY = event.global.y;
			dragOffsetX = event.global.x - this.container.x;
			dragOffsetY = event.global.y - this.container.y;
			this.hitbox.cursor = "grabbing";
		});

		this.hitbox.on("globalpointermove", (event) => {
			if (!this.dragging) return;
			const dx = Math.abs(event.global.x - startX);
			const dy = Math.abs(event.global.y - startY);
			if (!moved && (dx > 6 || dy > 6)) {
				moved = true;
				this.animations.play("drag");
				this.onDragged();
			}
			if (moved) {
				const nx = Math.max(40, Math.min(window.innerWidth - 40, event.global.x - dragOffsetX));
				const ny = Math.max(80, Math.min(this.floorY, event.global.y - dragOffsetY));
				this.container.x = nx;
				this.container.y = ny;
			}
		});

		const finishDrag = () => {
			if (!this.dragging) return;
			this.dragging = false;
			this.hitbox.cursor = "grab";

			if (!moved) {
				// 더블클릭 → 쓰다듬기(행복)
				const now = Date.now();
				if (now - this.lastClickAt < 420) {
					this.lastClickAt = 0;
					this.showMood("happy", 3400);
					this.callbacks.onDialogueTrigger?.(this, "happy", "happy");
				} else {
					this.lastClickAt = now;
				}
			} else {
				// 드래그 종료 후 표정 복귀 (짜증 타이머가 없으면 기본으로)
				if (this.moodTimerMs <= 0) {
					this.animations.play(this.behaviorMode === "manual" ? this.currentAction : "idle");
				}
			}
		};

		this.hitbox.on("pointerup", finishDrag);
		this.hitbox.on("pointerupoutside", finishDrag);
	}

	private onDragged() {
		const now = Date.now();
		this.recentDragTimes = this.recentDragTimes.filter((t) => now - t < 15_000);
		this.recentDragTimes.push(now);

		if (this.recentDragTimes.length >= 3) {
			// 짧은 시간에 3번 이상 잡아 끌면 짜증 반응
			this.recentDragTimes = [];
			this.showMood("annoyed", 4000);
			this.callbacks.onDialogueTrigger?.(this, "annoyed", "annoyed");
			this.lastDragLineAt = now;
			return;
		}

		if (now - this.lastDragLineAt > 4000) {
			this.lastDragLineAt = now;
			this.callbacks.onDialogueTrigger?.(this, "drag", "normal");
		}
	}

	walkTo(x: number, onArrive?: () => void) {
		const clamped = Math.max(50, Math.min(window.innerWidth - 50, x));
		this.targetX = clamped;
		this.onArriveCallback = onArrive ?? null;
		this.faceToward(clamped);
		this.currentAction = "walk";
		this.animations.play("walk");
	}

	faceToward(x: number) {
		if (Math.abs(x - this.container.x) < 2) return;
		this.setDirection(x < this.container.x ? "left" : "right");
	}

	moveTo(x: number) {
		this.container.x = Math.max(40, Math.min(window.innerWidth - 40, x));
	}

	update(deltaMS: number) {
		const deltaSeconds = Math.min(0.1, deltaMS / 1000);

		// 바닥 중력 낙하 (공중에 놓았을 때 자연스럽게 내려옴)
		if (!this.dragging && this.container.y < this.floorY) {
			this.fallVelocity += 1800 * deltaSeconds;
			this.container.y = Math.min(this.floorY, this.container.y + this.fallVelocity * deltaSeconds);
			if (this.container.y >= this.floorY) {
				this.fallVelocity = 0;
			}
		}

		if (this.moodTimerMs > 0) {
			this.moodTimerMs = Math.max(0, this.moodTimerMs - deltaMS);
			if (this.moodTimerMs === 0 && !this.dragging && !this.peerChatting && !this.userChatting) {
				this.currentAction = "idle";
				this.animations.play("idle");
			}
		}

		// 목표 지점으로 걷기 (자율 행동 또는 시메지 간 대화 이동 모두 처리)
		if (!this.dragging && this.targetX !== null) {
			const distance = this.targetX - this.container.x;
			const dir = Math.sign(distance);
			const movement = this.speed * deltaSeconds;

			if (Math.abs(distance) <= movement) {
				this.container.x = this.targetX;
				this.targetX = null;
				const cb = this.onArriveCallback;
				this.onArriveCallback = null;
				if (!this.peerChatting && this.moodTimerMs <= 0) {
					this.currentAction = "idle";
					this.animations.play("idle");
				}
				cb?.();
			} else {
				this.container.x += dir * movement;
			}
			return;
		}

		if (this.behaviorMode === "manual") {
			return;
		}

		this.behaviorController.update(deltaMS);
	}

	setManualBehavior(behavior: BehaviorType) {
		this.behaviorMode = "manual";
		this.moodTimerMs = 0;
		this.setBehavior(behavior);
	}

	resumeAutomaticBehavior() {
		this.behaviorMode = "automatic";
		this.moodTimerMs = 0;
		this.setBehavior("idle");
	}

	setPosition(x: number, y: number) {
		this.container.x = x;
		this.container.y = y;
	}

	private setBehavior(behavior: BehaviorType) {
		this.currentAction = behavior;
		this.targetX = null;
		this.onArriveCallback = null;
		this.animations.play(behavior);
	}

	setDirection(direction: "left" | "right") {
		this.direction = direction;
		const scale = Math.abs(this.container.scale.x) || 1;
		this.container.scale.x = direction === "left" ? scale : -scale;
	}

	setAction(action: BehaviorType) {
		this.currentAction = action;
		this.targetX = null;
		this.animations.play(action);
	}

	destroy() {
		this.onArriveCallback = null;
		this.container.destroy({ children: true });
	}
}
