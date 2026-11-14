interface GameUIState {
	activePanel: "story" | "map" | "inventory" | null;

	isToolPanelOpen: boolean;
	isCharacterPanelOpen: boolean;

	selectedCharacterId: string | null;
	selectedInteractableId: string | null;
}

class createGameUIStore {
	#store = $state<GameUIState>();
	get value() {
		return this.#store;
	}
}

export const gameUIlStore = new createGameUIStore();
