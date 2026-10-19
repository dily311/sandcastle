import type { Behavior } from "../simejiType";
import type { Character } from "./Character";

export class BehaviorController {
	private character: Character;
	private behaviors: Behavior[];
	private timer = 0;
	private nextDecision = 3000;

	constructor(character: Character, behaviors: Behavior[]) {
		this.character = character;
		this.behaviors = behaviors;
		this.scheduleNext();
	}

	update(deltaMS: number) {
		if (this.character.behaviorMode === "manual") {
			return;
		}
		this.timer += deltaMS;

		if (this.timer < this.nextDecision) {
			return;
		}

		this.timer = 0;
		this.decide();
		this.scheduleNext();
	}

	private scheduleNext() {
		this.nextDecision = this.random(1000, 3000);
	}

	private decide() {
		const behavior = this.pickWeightedBehavior();
		this.execute(behavior);
	}

	private execute(behavior: Behavior) {
		switch (behavior.type) {
			case "idle":
				this.character.setAction("idle");
				break;

			case "walk":
				this.walkRandom();
				break;

			case "action":
				this.character.setAction(behavior.type);
				console.log("행동:", behavior.name);
				break;
		}
	}

	private walkRandom() {
		/**
		 * 실제 범위는 WidgetGame에서
		 * 전달받도록 나중에 개선 가능
		 */
		const targetX = this.random(50, window.innerWidth - 50);
		this.character.walkTo(targetX);
	}

	private pickWeightedBehavior(): Behavior {
		const totalWeight = this.behaviors.reduce((sum, behavior) => sum + behavior.weight, 0);
		let random = Math.random() * totalWeight;

		for (const behavior of this.behaviors) {
			random -= behavior.weight;

			if (random <= 0) {
				return behavior;
			}
		}

		return this.behaviors[0];
	}

	private random(min: number, max: number) {
		return Math.random() * (max - min) + min;
	}
}
