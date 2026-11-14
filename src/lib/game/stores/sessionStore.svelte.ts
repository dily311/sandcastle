import { gameEngineCommands } from "$/ipc/commands";
import type { GamePlayerSessions } from "$/lib/type/game";

class createSessionStore {
	#store = $state<GamePlayerSessions[]>();
	private current = $state<string | null>(null);
	get value() {
		return this.#store;
	}

    async loadSessionList() {
        this.#store = await gameEngineCommands.listSessions();
		
    }

	setSession(id: string) {
		this.current = id;
	}
}

export const sessionStore = new createSessionStore();
