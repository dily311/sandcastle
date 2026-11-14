// 지금 열린 게임의 정적 데이터 (open_game 응답: 프로젝트·view·에셋 주소)
import { call, commands } from "$shared/ipc/client";
import type { OpenGameResponse } from "../types";

class GameStore {
	#store = $state<OpenGameResponse>();

	setBundle(value: OpenGameResponse) {
		// 해시가 같으면 같은 데이터
		if (this.#store?.contentHash === value.contentHash) return;
		this.#store = value;
	}

	async loadBundle(gameId: string) {
		this.#store = (await call(commands.openGame(gameId), { source: "game" })) as unknown as OpenGameResponse;
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

export const gameStore = new GameStore();

/** 게임 폴더 기준 상대 경로 → 불러올 수 있는 주소 */
export const assetUrl = (path: string) => (gameStore.assetBaseUrl ?? "") + path.split("/").map(encodeURIComponent).join("/");
