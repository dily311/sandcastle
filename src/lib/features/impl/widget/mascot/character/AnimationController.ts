import { AnimatedSprite, Assets, Container, Texture } from "pixi.js";
import type { BehaviorType, CharacterAnimation } from "../simejiType";

export class AnimationController {
	sprite!: AnimatedSprite;
	private animations: Record<BehaviorType, Texture[]> = {
		idle: [],
		sleep: [],
		walk: [],
		sitting: [],
		working: [],
		action: []
	};

	// 프레임
	private timer = 0;

	// 현재 애니메이션
	private currentAnimation: BehaviorType = "idle";

	async init(assetPath: string, container: Container) {
		try {
			// 애니메이션 에셋 로드
			const sheet = await Assets.load(assetPath);

			this.animations.idle = [
				sheet.textures["idle_0"],
				sheet.textures["idle_1"],
				sheet.textures["idle_2"],
				sheet.textures["idle_3"],
			];
			this.animations.walk = [
				// sheet.textures["walk_0"],
				sheet.textures["walk_1"],
				sheet.textures["walk_2"],
				sheet.textures["walk_3"],
			];
			this.animations.sitting = [sheet.textures["sitting_0"]];
			this.animations.working = [sheet.textures["working_0"]];

			// 스프라이트 생성
			this.sprite = new AnimatedSprite(this.animations.idle);

			// 아래 중앙을 기준으로 스프라이트 표시
			this.sprite.anchor.set(0.5, 1);
			this.sprite.scale.set(0.5);

			// 애니메이션 속도
			this.sprite.animationSpeed = 0.02;

			container.addChild(this.sprite);

			this.sprite.play();	
		} catch (error) {
			console.error(`[Mascot] 에셋 로드 실패 ${error}`);
		}
	}

	play(animation: BehaviorType) {
		// 현재 애니메이션과 동일하다면 리턴
		if (this.currentAnimation === animation) return;

		this.currentAnimation = animation;
		this.sprite.textures = this.animations[animation];
		// 0부터 플레이 (이미지를 한장씩 불러오는 방식에 해당)
		// 시트로 하는 경우에는 표시된 이미지 범위를 이동해야 함
		// 관련 메소드가 있을까?
		this.sprite.gotoAndPlay(0);
	}
}
