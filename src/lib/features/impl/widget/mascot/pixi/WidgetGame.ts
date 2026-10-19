import { Application } from "pixi.js";
import { CharacterManager } from "./CharaterManager";
import { behaviors } from "../behaviors";
import { pixiInteractive } from "$/lib/interaction/interactive-pixi";
import { handleContextMenuForMascot } from "$/lib/contextmenu/contextMenu.svelte";
import { WIDGET_MASCOT_FEATURE_ID } from "../mascotFeature.svelte";
import type { Character } from "../character/Character";

const example = "/src/data/widget/character_sheet.json";

export class WidgetGame {
	app: Application;
	
	characters = new CharacterManager();
	private running = false;
	stopInteractive?: () => void ;

	constructor() {
		this.app = new Application();
	}

	async init(container: HTMLElement) {
		await this.app.init({
            // 컨테이너 크기에 맞춤
			resizeTo: container,

			// 투명배경
			backgroundAlpha: 0,

			/**
			 * 위젯은 성능을 위해
			 * 별도 ticker 사용
			 */
			sharedTicker: false,
			autoStart: true,
		});

		container.appendChild(this.app.canvas);

		// 캐릭터 로드
		const char = await this.characters.spawnCharacter("1", this.app.screen.width / 2, this.app.screen.height, example, behaviors);
		this.stopInteractive = pixiInteractive(this.app, char.hitbox, "widget.mascot");

		// 이벤트
		char.hitbox.on("rightclick", (e) => handleRightClick(e, char), char);

		// 캐릭터와 스테이지를 연결
		this.app.stage.addChild(char.container);

		// 매 프레임 업데이트
		this.app.ticker.add(this.update);

		this.running = true;
	}

	private update = () => {
		if (!this.running) return;

		/**
		 * 여기에 이동/애니메이션 로직
		 */
		const deltaMS = this.app.ticker.deltaMS;

		// 캐릭터 이동
		this.characters.update(deltaMS);
	};

	pause() {
		this.running = false;
		this.app.stop();
		this.stopInteractive?.();
	}

	start() {
		this.running = true;
        this.app.ticker.start();
		this.app.start();
	}

	destroy() {
		this.running = false;

        this.app.ticker.stop();
		this.app.destroy({
			removeView: true,
		});
		this.stopInteractive?.();
	}
}

function handleRightClick(event: any, char: Character) {
	const global = event.global;
	handleContextMenuForMascot(global, WIDGET_MASCOT_FEATURE_ID,{data: {char: char}});
}

