import type { WorldInfoBook, WorldInfoEntry } from "$/lib/type/conv";
import { dbCommands } from "$ipc/commands";

class WorldInfoStore {
	#store = $state<WorldInfoBook[]>([]);

	get value() {return this.#store};

	async loadAll() {
		const worldInfoBooks = await dbCommands.getWorldInfoBooks();
		this.#store = worldInfoBooks;
	}

	async create(input: { id?: string, name: string; description: string | null }) {
		const book = await dbCommands.createWorldInfoBook({ input });
		this.#store.push(book);
		return book;
	}

	async remove(id: string) {
		await dbCommands.deleteWorldInfoBook({ id });
		this.#store = this.#store.filter((w) => w.id !== id);
	}

	async getWorldInfoEntries(bookId: string) {
		return await dbCommands.getWorldInfoEntries({ bookId });
	}

	async upsertWorldInfoEntry(input: Partial<WorldInfoEntry> & { bookId: string; content: string | null }) {
		return await dbCommands.upsertWorldInfoEntry({ input });
	}

	async deleteWorldInfoEntry(worldInfoEntryId: string) {
		await dbCommands.deleteWorldInfoEntry({ id: worldInfoEntryId });
	}
}

export const worldInfos = new WorldInfoStore();
