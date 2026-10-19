import type { Persona } from "$/lib/type/conv";
import { dbCommands } from "$ipc/commands";
import { SEED_PERSONAS } from "$/data/seed";
import { appSetting } from "../stores/settingStore.svelte";
import { confirm } from "@tauri-apps/plugin-dialog";

class PersonaStore {
	#store = $state<Persona[]>([]);

	get value() {
		return this.#store;
	}
	get activePersona() {
		return this.#store.find((persona) => persona.id === appSetting.selectedPersonaId);
	}

	async loadAll() {
		const data = await dbCommands.getPersonas();
		if (data.length === 0) {
			SEED_PERSONAS.map(async (p, index) => {
				const persona = await this.create(p);
				if (index == 0) {
					appSetting.update({ selectedPersonaId: persona.id });
				}
			});
		}
		this.#store = data;
	}

	async create(input: Omit<Persona, "id" | "createdAt" | "updatedAt">) {
		const persona = await dbCommands.createPersona({ input });
		this.#store.push(persona);
		return persona;
	}

	async newPersona() {
		const persona = await this.create({
			name: "User",
			avatar: "",
			description: "",
			worldInfoIds: [],
			tags: [],
			color: "#6366f1",
			isSystemDefault: false,
		});
		appSetting.update({ selectedPersonaId: persona.id });
		return persona;
	}

	async patch(id: string, changes: Partial<Persona>) {
		const current = this.#store.find((c) => c.id === id);
		if (!current) return;
		const merged = { ...current, ...changes, updatedAt: Date.now() };
		await dbCommands.updatePersona({ id, input: merged });
		this.#store = this.#store.map((c) => (c.id === id ? merged : c));
	}

	async remove(id: string) {
		await dbCommands.deletePersona({ id });
		this.#store = this.#store.filter((c) => c.id !== id);
	}

	async setSystemDefaultPersona(id: string) {
		await dbCommands.setSystemDefaultPersona({ id });
		appSetting.update({ selectedPersonaId: id });
	}
}

export const personas = new PersonaStore();

// PersonaEditor.svelte 또는 삭제 버튼 핸들러
export async function handleDeletePersona(id: string) {
	const usage = await dbCommands.getPersonaUsage({ id });

	if (usage.characterCount > 0 || usage.isSystemDefault) {
		const parts = [];
		if (usage.isSystemDefault) parts.push("시스템 기본 페르소나로 지정됨");
		if (usage.characterCount > 0) parts.push(`${usage.characterCount}개 캐릭터의 기본값으로 사용 중`);

		const confirmed = await confirm(
			`이 페르소나는 ${parts.join(", ")}입니다. 삭제하면 해당 설정이 초기화됩니다. 계속하시겠어요?`,
		);
		if (!confirmed) return;
	}

	await personas.remove(id);
}
