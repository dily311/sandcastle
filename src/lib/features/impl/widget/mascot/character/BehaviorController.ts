import { tr } from "$lib/utils/i18n";
import type { Behavior } from "../simejiType";
import type { Character } from "./Character";

export class BehaviorController {
	private character: Character;
	private behaviors: Behavior[];
	private intervalSec: number;
	private timer = 0;
	private nextDecision = 4000;

	constructor(character: Character, behaviors: Behavior[], intervalSec = 5) {
		this.character = character;
		this.behaviors = behaviors;
		this.intervalSec = Math.max(1.5, intervalSec);
		this.scheduleNext();
	}

	updateConfig(behaviors: Behavior[], intervalSec: number) {
		this.behaviors = behaviors;
		this.intervalSec = Math.max(1.5, intervalSec);
	}

	update(deltaMS: number) {
		if (this.character.behaviorMode === "manual" || this.character.isBusy()) {
			return;
		}
		this.timer += deltaMS;
		if (this.timer < this.nextDecision) return;

		this.timer = 0;
		const chosen = this.pickWeightedBehavior();
		this.execute(chosen);
		this.scheduleNext(chosen);
	}

	private scheduleNext(behavior?: Behavior) {
		if (behavior) {
			this.nextDecision = this.random(behavior.minDuration, behavior.maxDuration);
			return;
		}
		const base = this.intervalSec * 1000;
		this.nextDecision = this.random(base * 0.6, base * 1.4);
	}

	private execute(behavior: Behavior) {
		switch (behavior.type) {
			case "idle":
				this.character.setAction("idle");
				break;
			case "walk":
				this.walkRandom();
				break;
			case "sitting":
				this.character.setAction("sitting");
				break;
			case "sleep":
				this.character.setAction("sleep");
				break;
			case "working":
				this.character.setAction("working");
				// 활동 중 가끔 혼잣말 출력
				if (Math.random() < 0.65) {
					this.character.triggerIdleChatter();
				}
				break;
			default:
				this.character.setAction("idle");
				break;
		}
	}

	private walkRandom() {
		const margin = 80;
		const maxW = Math.max(margin * 2, window.innerWidth - margin);
		const targetX = this.random(margin, maxW);
		this.character.walkTo(targetX);
	}

	private pickWeightedBehavior(): Behavior {
		const valid = this.behaviors.filter((b) => b.weight > 0);
		if (valid.length === 0) {
			return this.behaviors[0] ?? { id: "idle", name: tr("mascotLib.behaviors.idle"), type: "idle", weight: 1, minDuration: 3000, maxDuration: 5000 };
		}
		const totalWeight = valid.reduce((sum, b) => sum + b.weight, 0);
		let r = Math.random() * totalWeight;
		for (const b of valid) {
			r -= b.weight;
			if (r <= 0) return b;
		}
		return valid[0];
	}

	private random(min: number, max: number) {
		return Math.random() * (max - min) + min;
	}
}
