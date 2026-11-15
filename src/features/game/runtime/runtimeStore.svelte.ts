// 지금 플레이 중인 세션의 상태 (continue_game / dispatch_command 응답)
import { call, commands } from "$shared/ipc/client";
import type { ContinueGameResponse, GameCommand } from "../types";
import { getConnectionRequirement } from "../play/vn/playUtils";
import { gameStore } from "./gameStore.svelte";
import { reportGameError } from "../errors";

const src = { source: "game" };

class RuntimeStore {
	#store = $state<ContinueGameResponse>();

	async initialize(value: ContinueGameResponse) {
		this.#store = value;
	}

	async loadState(sessionId: string) {
		this.#store = (await call(commands.continueGame(sessionId, null), src)) as unknown as ContinueGameResponse;
	}

	async setState(value: Partial<ContinueGameResponse>) {
		if (!this.#store) return;
		this.#store = { ...this.#store, ...value };
	}

	async dispatchCommand(command: GameCommand) {
		// 규칙상 막힌 행동은 경고로, 플레이가 닫혔으면 돌아가는 버튼과 함께 알린다
		const res = await call(commands.dispatchCommand(command as never), { ...src, silent: true }).catch((e) => {
			reportGameError(e, gameStore.project?.id);
			throw e;
		});
		this.#store = { ...(this.#store ?? { history: { entries: [], hasMore: false, oldestSeq: 0 } }), ...(res as unknown as ContinueGameResponse) };
	}

	get value() {
		return this.#store;
	}
	get state() {
		return this.#store?.state;
	}
	get stage() {
		return this.#store?.state.stage;
	}
	get events() {
		return this.#store?.events;
	}
	get gameTime() {
		return this.#store?.state.gameTime;
	}
	get weatherId() {
		return this.#store?.state.weatherId;
	}
	get timePeriodId() {
		return this.#store?.state.timePeriodId;
	}
	get currentSceneId() {
		return this.#store?.state.currentSceneId;
	}
	get currentLocationId() {
		return this.#store?.state.locationId;
	}
	get availableChoices() {
		return this.#store?.state.availableChoices;
	}
	get availableMoves() {
		return this.#store?.state.availableMoves;
	}
	get flags() {
		return this.#store?.state.flags;
	}
	get inventory() {
		return this.#store?.state.inventory;
	}
	get relationships() {
		return this.#store?.state.relationships;
	}
	get activeNpcId() {
		return this.#store?.state.activeNpcId;
	}
	get presentNpcs() {
		return this.#store?.state.presentNpcs;
	}

	// 현재 상태 관리
	get destinations() {
		return (
			gameStore.view?.connections
				.filter((c) => c.fromId === this.state?.locationId)
				.map((c) => ({
					...c,
					location: gameStore.view?.locations.find((l) => l.id === c.toId),
					requirement: this.state?.availableMoves.find((l) => l.toId == c.fromId),
				}))
				.filter((i) => i.location) ?? []
		);
	}
	get mapZones() {
		return (gameStore.view?.zones ?? [])
			.filter((z) => z.mapId === this.state?.mapId)
			.map((z) => ({ ...z, locations: (gameStore.view?.locations ?? []).filter((l) => l.zoneId === z.id) }));
	}

	get currentMap() {
		return gameStore.view?.maps.find((m) => m.id === this.state?.mapId);
	}

	get currentLocation() {
		return gameStore.view?.locations.find((l) => l.id === this.currentLocationId);
	}

	get currentWeather() {
		return gameStore.view?.weatherTypes.find((w) => w.id === this.weatherId);
	}

	get currentBg() {
		return this.stage?.background;
	}
	get currentBgm() {
		return this.stage?.bgm;
	}

	get currentScene() {
		return gameStore.scenes?.find((s) => s.id === this.currentSceneId);
	}
	get currentChoices() {
		return this.currentScene?.choices.filter((c) => this.availableChoices?.includes(c.id)) ?? [];
	}

	get inventoryEntries() {
		return Object.entries((runtimeStore.inventory ?? {}) as Record<string, number>)
			.filter(([, quantity]) => typeof quantity === "number" && quantity > 0)
			.map(([itemId, quantity]) => ({
				itemId,
				quantity,
				item: gameStore.view?.items.find((i) => i.id === itemId),
			}));
	}

	get nearbyCharacters() {
		return (gameStore.view?.characters ?? []).filter((c) => runtimeStore.presentNpcs?.includes(c.id));
	}

	get localInteractables() {
		return (gameStore.view?.interactables ?? []).filter((i) => i.locationId === runtimeStore.currentLocationId);
	}
	get canWait() {
		return !this.currentSceneId;
	}
}

export const runtimeStore = new RuntimeStore();
