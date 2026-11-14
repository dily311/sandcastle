import type { GameJournal } from "$/lib/type/game";

class createJournalStore {
	#store = $state<GameJournal[]>();
	get value() {
		return this.#store;
	}

	setJournal(value: GameJournal[]) {
		this.#store = value;
	}
}

export const journalStore = new createJournalStore();
