import { Container, Graphics, Sprite } from "pixi.js";
import { AnimationController } from "./AnimationController";
import { BehaviorController } from "./BehaviorController";
import type { Behavior, BehaviorMode, BehaviorType } from "../simejiType";

export class Character {
	container = new Container();
	sprite!: Sprite;
	hitbox!: Graphics;
	behaviorController!: BehaviorController;
	animations = new AnimationController();
	behaviorMode: BehaviorMode = "automatic";

	speed = 80;
	direction: "left" | "right" = "right";
	currentAction: BehaviorType = "idle";
	targetX: number | null = null;

	async init(assetPath: string, behaviors: Behavior[]) {
		// 애니메이션 등록
		this.animations.init(assetPath, this.container);

		// 컨트롤러와 연결
		this.behaviorController = new BehaviorController(this, behaviors);

		// 히트 박스 생성
		this.hitbox = new Graphics();
		this.hitbox.rect(-50, -180, 100, 180).fill({ alpha: 0 });
		this.container.addChild(this.hitbox);
		this.hitbox.eventMode = "static";
		// this.hitbox.cursor = "pointer";
		this.handleDrag();
	}

	handleDrag() {
		let dragging = false;
		let dragOffsetX = 0;

		this.hitbox.cursor = "grab";

		this.hitbox.on("pointerdown", (event) => {
			dragging = true;
			dragOffsetX = event.global.x - this.container.x;
			this.container.cursor = "grabbing";
		});

		this.hitbox.on("globalpointermove", (event) => {
			if (!dragging) return;

			this.moveTo(event.global.x - dragOffsetX);
		});

		this.hitbox.on("pointerup", () => {
			dragging = false;
			this.container.cursor = "grab";
		});

		this.hitbox.on("pointerupoutside", () => {
			dragging = false;
			this.container.cursor = "grab";
		});
	}
	
	walkTo(x: number) {
		this.targetX = x;

		if (x < this.container.x) {
			this.setDirection("left");
		} else {
			this.setDirection("right");
		}

		this.animations.play("walk");
	}

	moveTo(x: number) {
		this.container.x = x;
	}

	update(deltaMS: number) {
		if (this.behaviorMode === "manual") {
			return;
		}

		// 랜덤 행동
		this.behaviorController.update(deltaMS);

		const deltaSeconds = deltaMS / 1000;
		if (this.targetX === null) {
			return;
		}

		const distance = this.targetX - this.container.x;
		const direction = Math.sign(distance);
		const movement = this.speed * deltaSeconds;

		/**
		 * 목적지에 도착
		 */
		if (Math.abs(distance) <= movement) {
			this.container.x = this.targetX;
			this.targetX = null;
			this.animations.play("idle");
			return;
		}

		this.container.x += direction * movement;
	}

	setManualBehavior(behavior: BehaviorType) {
		this.behaviorMode = "manual";
		this.setBehavior(behavior);
	}

	resumeAutomaticBehavior() {
		this.behaviorMode = "automatic";
		this.setBehavior("idle");
	}

	setPosition(x: number, y: number) {
		this.container.x = x;
		this.container.y = y;
	}

	private setBehavior(behavior: BehaviorType) {
		this.currentAction = behavior;

		// 기존 이동 취소
		this.targetX = null;
		this.animations.play(behavior);
	}
	
	setDirection(direction: "left" | "right") {
		this.direction = direction;

		const scale = Math.abs(this.container.scale.x);
		/**
		 * 좌우 반전
		 */
		this.container.scale.x = direction === "left" ? scale : -scale;
	}

	setAction(action: BehaviorType) {
		this.currentAction = action;
	}

	destroy() {
		this.container.destroy();
	}
}
