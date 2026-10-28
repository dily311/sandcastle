import { gameManagerCommands } from "$/ipc/commands";
import { push } from "svelte-spa-router";
import type { GameProject } from "../type/game";

class createGameManagerStore {
	#store = $state<GameProject[]>();

	async loadAll() {
		this.#store = await gameManagerCommands.getGameList();
	}

    add() {

    }

	async create() {
        const title = "";
		const newGameId = await gameManagerCommands.createGame({ title, templateId: "default_template" });
        return newGameId;
	}

	async delete(gameId: string, title: string) {
		if (!confirm(`"${title}" 게임을 정말 삭제하시겠습니까? 관련 데이터가 모두 사라집니다.`)) {
			return;
		}

		await gameManagerCommands.deleteGame({ gameId });
		this.#store = this.#store?.filter((g) => g.id !== gameId);
	}

	get value() {
		return this.#store;
	}
}

export const gameManager = new createGameManagerStore();

// -----------------------------------
// 사용함수
// -----------------------------------

// 새로만들기 버튼 클릭 시 (create_game 커맨드 연동 예시)
export async function handleCreateNewGame() {
    const newGameId = gameManager.create();
	// 생성 후 바로 에디터로 이동
	push(`/game/${newGameId}/editor`);
}

export async function handleOpenGame(gameId: string) {
	await gameManagerCommands.openGame({ gameId });
	push(`/game/${gameId}/sessions`);
}

export function handleEditGame(gameId: string) {
	// svelte-spa-router 라우팅 이동
	push(`/game/${gameId}/editor`);
}
