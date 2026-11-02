import type { Character, CharacterInput } from "$/lib/type/conv";
import { dbCommands } from "$ipc/commands";

class CharacterStore {
	#store = $state<Character[]>([]);
	get value() {
		return this.#store;
	}
	get favorites() {
		return this.#store.filter(c => c.favorite);
	}

	getChar(id: string) {
		return this.#store.find((c) => c.id === id);
	}

	async loadAll() {
		this.#store = await dbCommands.getCharacters();
	}

	async create(input: CharacterInput) {
		const c = await dbCommands.createCharacter({ input });
		this.#store.push(c);
		return c;
	}

	async duplicate(id: string) {
		const target = this.#store.find((c) => c.id === id);
		if (!target) return;
		const newChar = await dbCommands.createCharacter({input: target});
		this.#store.push(newChar);
		return newChar;
	}

	async toggleFavorite(id: string) {
		const target = this.#store.find((c) => c.id === id);
		if (!target) return;
		this.#store = this.#store.map(((c) => (c.id === id) ? {...target, favorite: !target.favorite} : c));
		await dbCommands.toggleFavoriteCharacter({id, isFavorite: !target.favorite});
	}

	async patch(id: string, changes: Partial<Character>) {
		const current = this.#store.find((c) => c.id === id);
		if (!current) return;
		const merged = { ...current, ...changes, updatedAt: Date.now() };
		await dbCommands.updateCharacter({ id, input: merged });
		this.#store = this.#store.map((c) => (c.id === id ? merged : c));
	}

	async remove(id: string) {
		await dbCommands.deleteCharacter({ id });
		this.#store = this.#store.filter((c) => c.id !== id);
	}

	async updateMessageCount(id: string, delta: number) {
		const current = this.#store.find((c) => c.id === id);
		if (!current) return;

		const newCount = Math.max(0, (current.messageCount || 0) + delta);
		this.#store = this.#store.map((c) =>
			c.id === id ? { ...current, messageCount: newCount } : c,
		);
	}
}

export const characters = new CharacterStore();
