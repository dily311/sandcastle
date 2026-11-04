import { gameEngineCommands, gameManagerCommands } from "$/ipc/commands";
import { push } from "svelte-spa-router";
import type {
	GameCommand,
	GameEvent,
	GameJournal,
	GamePlayerSessions,
	GameResponse,
	OpenGameResponse,
	RenderState,
} from "../type/game";
import { convertFileSrc } from "@tauri-apps/api/core";

/** 런타임 상태의 게임을 관리하는 스토어
 * 현재 플레이하는 게임 상태를 관리하므로 정적 데이터(OpenGameResponse)는 항상 들고 있어야 함
 * 플레이어 세션은 다른 스토어에서 관리하는 걸로
 */
class GameStore {
	#store = $state<OpenGameResponse | null>(null);
	currentSessionId = $state<string | null>(null);
	renderState = $state<GameResponse>();
	journals = $state<GameJournal[]>([]);

	get view() {
		return this.#store?.view;
	}

	get state() {
		return this.renderState?.state;
	}

	get currentMap() {
		return this.#store?.view.maps.find((m) => m.id === this.state?.mapId);
	}
	get currentZone() {
		return this.#store?.view.zones.find((m) => m.id === this.state?.zoneId);
	}
	get currentLocation() {
		return this.#store?.view.locations.find((m) => m.id === this.state?.locationId);
	}
	get currentWeather() {
		return this.#store?.view.weatherTypes.find((m) => m.id === this.state?.weatherId);
	}
	get gameTime() {
		return this.state?.gameTime;
	}
	get activeNpc() {
		return this.#store?.view.characters.filter((c) => this.state?.presentNpcs.includes(c.id));
	}

	get stage() {
		return this.state?.stage;
	}

	get bg() {
		const current = this.#store?.view.assets.backgrounds.find((b) => b.id === this.stage?.background.id);

		if (current) {
			return convertFileSrc(current.path);
		}
	}

	get locations() {
		const destinations = this.#store?.view.connections
			.filter((connection) => connection.fromId === this.state?.locationId)
			.map((connection) => ({
				...connection,
				location: this.#store?.view.locations.find((location) => location.id === connection.toId),
			}))
			.filter((item) => item.location);
		return destinations;
	}

	get currentScene() {
		return this.view?.scenes.find((scene) => scene.id === this.state?.currentSceneId);
	}
	get choices() {
		return this.currentScene?.choices.filter((choice) => this.state?.availableChoices.includes(choice.id));
	}

	getNpcAssets(characterId: string): string {
		const standings = this.stage?.standings;
		if (!standings || !standings[characterId]) return "";

		return convertFileSrc(standings[characterId]?.asset.path);
	}

	updateStore(part: Partial<OpenGameResponse>) {
		if (!this.#store) return;
		this.#store = { ...this.#store, ...part };
	}

	/** 게임 선택 시 불러오는 데이터 */
	async loadGame(gameId: string) {
		const gameData = await gameManagerCommands.openGame({ gameId });
		this.#store = gameData;
	}

	async loadJournals() {

	}

	/** 게임 정적 데이터를 다시 불러올 때 사용 */
	async getGameView() {
		if (!this.#store) return;
		const res = await gameEngineCommands.getGameView();
		this.updateStore({ view: res });
	}

	async newGame() {
		if (!this.#store) return;
		const res = await gameEngineCommands.startNewGame();
		this.renderState = res;
	}

	async continueGame(sessionId: string) {
		if (!this.#store) return;
		const res = await gameEngineCommands.continueGame({ sessionId });
		this.renderState = res;
	}

	async dispatchCommand(command: GameCommand) {
		const res = await gameEngineCommands.dispatchGameCommand({ command });
		this.renderState = res;
	}
}

export const gameStore = new GameStore();

// ---------------------------------------------------------
// 사용 함수
// ---------------------------------------------------------
export function handleOpenGameDetailPage(id: string) {
	push(`/game/${id}`);
}

export async function handleStartGamePlay(id: string) {
	// 게임 데이터 미리 로드
	await gameStore.loadGame(id);

	push(`/game/${id}/sessions`);
}

export async function handleOpenGameEditor(id: string) {
	push(`/game/${id}/editor`);
}
