import { AnimatedSprite, Assets, Container, Graphics, Texture } from "pixi.js";
import type { MascotImages_Serialize } from "$ipc/bindings";
import { fileSrc } from "$shared/ipc/file";
import type { BehaviorType, MascotImageSlot } from "../simejiType";

const BASE_DISPLAY_HEIGHT = 140;
const FALLBACK_SIZE = 96;

async function loadImageTexture(path: string): Promise<Texture | null> {
	const src = fileSrc(path);
	if (!src) return null;
	try {
		const loaded = await Assets.load<Texture>(src);
		if (loaded instanceof Texture) return loaded;
	} catch {
		// Pixi Assets.load가 data: / asset: URL에서 실패할 경우 HTMLImageElement로 폴백
	}
	return new Promise((resolve) => {
		const img = new Image();
		img.crossOrigin = "anonymous";
		img.onload = () => {
			try {
				resolve(Texture.from(img));
			} catch {
				resolve(null);
			}
		};
		img.onerror = () => resolve(null);
		img.src = src;
	});
}

export class AnimationController {
	sprite: AnimatedSprite | null = null;
	fallbackGraphic: Graphics | null = null;

	private animations: Record<BehaviorType, Texture[]> = {
		idle: [],
		walk: [],
		sitting: [],
		sleep: [],
		working: [],
		drag: [],
		happy: [],
		annoyed: [],
	};
	private currentAnimation: BehaviorType = "idle";
	private userScale = 1;
	private displayWidth = FALLBACK_SIZE;
	private displayHeight = FALLBACK_SIZE;

	async init(images: MascotImages_Serialize, container: Container, scale = 1, fallbackAvatarPath?: string | null) {
		this.userScale = Math.max(0.35, Math.min(2.5, scale || 1));
		this.clear(container);

		const slots: MascotImageSlot[] = ["idle", "walk", "sitting", "sleep", "drag", "happy", "annoyed", "working"];
		await Promise.all(
			slots.map(async (slot) => {
				const paths = images[slot] ?? [];
				const loaded = await Promise.all(paths.map((p) => loadImageTexture(p)));
				this.animations[slot] = loaded.filter((t): t is Texture => t !== null);
			}),
		);

		// idle이 비어 있으면 다른 슬롯의 첫 이미지 또는 연결된 캐릭터 아바타를 사용
		if (this.animations.idle.length === 0) {
			const firstAvailable = slots.map((s) => this.animations[s]).find((arr) => arr.length > 0);
			if (firstAvailable) {
				this.animations.idle = [firstAvailable[0]];
			} else if (fallbackAvatarPath) {
				const avatarTex = await loadImageTexture(fallbackAvatarPath);
				if (avatarTex) this.animations.idle = [avatarTex];
			}
		}

		if (this.animations.idle.length > 0) {
			this.sprite = new AnimatedSprite(this.animations.idle);
			this.sprite.anchor.set(0.5, 1);
			this.sprite.animationSpeed = 0.08;
			this.applyScale(this.animations.idle[0]);
			container.addChild(this.sprite);
			if (this.animations.idle.length > 1) {
				this.sprite.play();
			} else {
				this.sprite.gotoAndStop(0);
			}
		} else {
			// 업로드된 이미지가 하나도 없을 때의 기본 귀염 도형 마스코트
			this.fallbackGraphic = this.createFallbackMascot();
			this.fallbackGraphic.scale.set(this.userScale);
			this.displayWidth = FALLBACK_SIZE * this.userScale;
			this.displayHeight = FALLBACK_SIZE * this.userScale;
			container.addChild(this.fallbackGraphic);
		}
	}

	private createFallbackMascot(): Graphics {
		const g = new Graphics();
		// 몸통
		g.roundRect(-34, -84, 68, 76, 30).fill({ color: 0xf4e3d7 }).stroke({ color: 0x5c4b3c, width: 3 });
		// 귀
		g.circle(-22, -84, 12).fill({ color: 0xf4e3d7 }).stroke({ color: 0x5c4b3c, width: 3 });
		g.circle(22, -84, 12).fill({ color: 0xf4e3d7 }).stroke({ color: 0x5c4b3c, width: 3 });
		// 눈
		g.circle(-12, -52, 4).fill({ color: 0x3b2f25 });
		g.circle(12, -52, 4).fill({ color: 0x3b2f25 });
		// 볼터치
		g.circle(-20, -44, 5).fill({ color: 0xf29f9f, alpha: 0.65 });
		g.circle(20, -44, 5).fill({ color: 0xf29f9f, alpha: 0.65 });
		// 미소
		g.moveTo(-6, -42).quadraticCurveTo(0, -36, 6, -42).stroke({ color: 0x3b2f25, width: 2.5 });
		return g;
	}

	private applyScale(tex: Texture) {
		if (!this.sprite) return;
		const h = Math.max(1, tex.height);
		const w = Math.max(1, tex.width);
		const normalized = (BASE_DISPLAY_HEIGHT / h) * this.userScale;
		this.sprite.scale.set(normalized);
		this.displayWidth = Math.max(48, w * normalized);
		this.displayHeight = Math.max(48, h * normalized);
	}

	setScale(scale: number) {
		this.userScale = Math.max(0.35, Math.min(2.5, scale || 1));
		if (this.sprite && this.sprite.textures.length > 0) {
			const tex = this.sprite.textures[0];
			if (tex instanceof Texture) this.applyScale(tex);
		} else if (this.fallbackGraphic) {
			this.fallbackGraphic.scale.set(this.userScale);
			this.displayWidth = FALLBACK_SIZE * this.userScale;
			this.displayHeight = FALLBACK_SIZE * this.userScale;
		}
	}

	getDisplaySize() {
		return { width: this.displayWidth, height: this.displayHeight };
	}

	play(animation: BehaviorType) {
		if (this.currentAnimation === animation) return;
		this.currentAnimation = animation;

		if (!this.sprite) return;
		const frames = this.animations[animation].length > 0 ? this.animations[animation] : this.animations.idle;
		if (frames.length === 0) return;

		this.sprite.textures = frames;
		this.applyScale(frames[0]);
		if (frames.length > 1) {
			this.sprite.gotoAndPlay(0);
		} else {
			this.sprite.gotoAndStop(0);
		}
	}

	clear(container: Container) {
		if (this.sprite) {
			container.removeChild(this.sprite);
			this.sprite.destroy();
			this.sprite = null;
		}
		if (this.fallbackGraphic) {
			container.removeChild(this.fallbackGraphic);
			this.fallbackGraphic.destroy();
			this.fallbackGraphic = null;
		}
		this.currentAnimation = "idle";
	}
}
