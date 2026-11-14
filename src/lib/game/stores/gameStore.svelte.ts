import { gameManagerCommands } from "$/ipc/commands";
import type { OpenGameResponse } from "$/lib/type/game";

class createGameStore {
	#store = $state<OpenGameResponse>();

	setBundle(value: OpenGameResponse) {
		// 해쉬값이 같으면 동일한 데이터이므로 리턴
		if (this.#store?.contentHash === value.contentHash) return;
		this.#store = value;
	}

	async loadBundle(gameId: string) {
		this.#store = await gameManagerCommands.openGame({ gameId });
	}

	get value() {
		return this.#store;
	}
	get project() {
		return this.#store?.project;
	}
	get view() {
		return this.#store?.view;
	}
	get scenes() {
		return this.#store?.view.scenes;
	}
	get assets() {
		return this.#store?.view.assets;
	}
	get assetBaseUrl() {
		return this.#store?.assetBaseUrl;
	}
}

export const gameStore = new createGameStore();
