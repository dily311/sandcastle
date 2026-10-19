import { APICommands } from "$/ipc/commands";
import type { ApiProfile, ApiProfileInput } from "../type/aiService";
import { appSetting } from "./settingStore.svelte";

class APIProfileStore {
	private store = $state<ApiProfile[]>([]);
	isConnected = $state<boolean>(false);

	get value() {return this.store};
	get selectedApiProfile() {
		return appSetting.selectedApiProfile ? this.store.find((v) => v.id === appSetting.selectedApiProfile) : null;
	}

	async loadAll() {
		const profile = await APICommands.getAllApiProfile();
		this.store = profile;
	}

	async create(input: ApiProfileInput, apiKey: string) {
		const profile = await APICommands.createApiProfile({ input, apiKey });
		this.store.push(profile);
		return profile;
	}

	async save(id: string, input: ApiProfile, apiKey: string) {
		const current = this.store.find((c) => c.id === id);
		if (!current) return;
		await APICommands.updateApiProfile({ id, input, apiKey });
		this.store = this.store.map((a) => (a.id === id ? input : a));
	}

	async remove(id: string) {
		await APICommands.deleteApiProfile({ id });
		this.store = this.store.filter((c) => c.id !== id);
	}
}

export const apiProfiles = new APIProfileStore();
