// 캐릭터·대화·폴더와 대화 설정에 쓰는 목록(페르소나·프리셋·API). 화면들이 같이 쓴다.
import type {
	ApiProfiles,
	CharacterImage_Serialize as CharacterImage,
	Character_Serialize as Character,
	ConversationFolder,
	Conversation_Serialize as Conversation,
	Persona_Serialize as Persona,
	PromptPresets_Serialize as Preset,
} from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { toasts } from "$shared/stores/toast.svelte";

export type { Character, CharacterImage, Conversation, ConversationFolder, Persona, Preset, ApiProfiles };

const src = { source: "character" };

class Library {
	characters = $state<Character[]>([]);
	conversations = $state<Conversation[]>([]);
	folders = $state<ConversationFolder[]>([]);
	personas = $state<Persona[]>([]);
	presets = $state<Preset[]>([]);
	apis = $state<ApiProfiles[]>([]);
	/** 캐릭터별 이미지 에셋 */
	images = $state<Record<string, CharacterImage[]>>({});
	loaded = $state(false);
	private foldersLoaded = new Set<string>();

	async load() {
		const [characters, conversations, personas, presets, apis] = await Promise.all([
			call(commands.getAllCharacters(), src),
			call(commands.getAllConversations(), src),
			call(commands.getAllPersonas(), { ...src, silent: true }).catch(() => []),
			call(commands.listPromptPresets(), { ...src, silent: true }).catch(() => []),
			call(commands.getAllApiProfile(), { ...src, silent: true }).catch(() => []),
		]);
		Object.assign(this, { characters, conversations, personas, presets, apis });
		this.loaded = true;
	}

	character(id: string | null | undefined) {
		return this.characters.find((c) => c.id === id);
	}
	conversation(id: string | null | undefined) {
		return this.conversations.find((c) => c.id === id);
	}
	persona(id: string | null | undefined) {
		return this.personas.find((p) => p.id === id);
	}
	/** 최근 활동 순 (폰 대화는 폰 화면에서만 보임) */
	convsOf(charId: string) {
		return this.conversations.filter((c) => c.characterId === charId && c.mode !== "phone").sort((a, b) => b.updatedAt - a.updatedAt);
	}
	foldersOf(charId: string) {
		return this.folders.filter((f) => f.characterId === charId);
	}

	// ---- 대화 ----
	private put(conv: Conversation) {
		const i = this.conversations.findIndex((c) => c.id === conv.id);
		if (i >= 0) this.conversations[i] = conv;
		else this.conversations.unshift(conv);
	}
	private patch(id: string, p: Partial<Conversation>) {
		const c = this.conversation(id);
		if (c) Object.assign(c, p);
	}
	/** 메시지 개수·마지막 활동이 바뀐 뒤 다시 읽기 */
	async refresh(id: string) {
		const conv = await call(commands.getConversation(id), { ...src, silent: true }).catch(() => null);
		if (conv) this.put(conv);
	}

	/** 새 대화: 캐릭터 기본 페르소나, 첫 메시지(다른 첫 메시지는 스와이프로) */
	async createConversation(charId: string): Promise<Conversation> {
		const ch = this.character(charId);
		if (!ch) throw new Error("캐릭터 없음");
		const def = ch.defaultPersonaId ?? (await call(commands.getConvSettings(), { ...src, silent: true }).catch(() => null))?.defaultPersonaId ?? "default";
		const n = this.convsOf(charId).length + 1;
		const conv = await call(
			commands.createConversation({ title: `${ch.name} 대화 ${n}`, characterId: charId, personaId: def, worldInfoIds: [], mode: "chat" }),
			src,
		);
		const greetings = [ch.firstMessage, ...ch.alternateFirstMessage].filter((g): g is string => !!g?.trim());
		if (greetings.length) {
			const at = Date.now();
			await call(
				commands.appendMessage({
					conversationId: conv.id,
					seq: 1,
					role: "assistant",
					name: ch.name,
					content: greetings[0],
					characterId: ch.id,
					meta: {
						sender: { type: "assistant", name: ch.name, id: ch.id },
						...(greetings.length > 1
							? { swipes: greetings.map((g) => ({ content: g, meta: {}, createdAt: at, updatedAt: at })), swipeId: 0 }
							: {}),
					},
				}),
				src,
			);
		}
		await this.refresh(conv.id);
		return this.conversation(conv.id) ?? conv;
	}

	/** 폰 대화: 페르소나(폰 주인)와 캐릭터 한 쌍에 하나. 첫 인사 없이 빈 대화로 시작 */
	async phoneConversation(personaId: string, charId: string): Promise<Conversation> {
		// 캐릭터 폰 대화(meta.phone)는 빼고
		const found = this.conversations.find(
			(c) => c.mode === "phone" && c.personaId === personaId && c.characterId === charId && !(c.meta as { phone?: unknown } | null)?.phone,
		);
		if (found) return found;
		const ch = this.character(charId);
		const conv = await call(
			commands.createConversation({ title: ch?.name ?? "폰 대화", characterId: charId, personaId, worldInfoIds: [], mode: "phone" }),
			src,
		);
		this.put(conv);
		return conv;
	}

	/** 대화 meta에 키를 합침 (값이 null이면 지움) */
	/** 캐릭터에 붙은 로어북 바꾸기 (나머지 캐릭터 정보는 그대로) */
	async setCharacterWorldInfo(charId: string, bookIds: string[]) {
		const c = this.character(charId);
		if (!c) return;
		const { id: _id, messageCount: _m, createdAt: _c, updatedAt: _u, ...rest } = c;
		void _id;
		void _m;
		void _c;
		void _u;
		const saved = await call(commands.updateCharacter(charId, { ...rest, worldInfoIds: bookIds }), src);
		const i = this.characters.findIndex((x) => x.id === charId);
		if (i >= 0) this.characters[i] = saved;
	}

	/** 대화에 직접 붙인 로어북 (캐릭터·페르소나의 책과 별개) */
	async setWorldInfo(id: string, bookIds: string[]) {
		const conv = await call(commands.setConversationWorldinfo(id, bookIds), src);
		this.put(conv);
		return conv;
	}

	async patchMeta(id: string, patch: Record<string, unknown>) {
		const conv = await call(commands.patchConversationMeta(id, patch as never), src);
		this.put(conv);
		return conv;
	}

	async rename(id: string, title: string) {
		const t = title.trim();
		const c = this.conversation(id);
		if (!t || !c || c.title === t) return;
		await call(commands.renameConversation(id, t), src);
		this.patch(id, { title: t });
	}
	async togglePin(id: string) {
		const c = this.conversation(id);
		if (!c) return;
		await call(commands.setConversationPinned(id, !c.pinned), src);
		this.patch(id, { pinned: !c.pinned });
	}
	async move(id: string, folderId: string | null) {
		await call(commands.moveConversation(id, folderId), src);
		this.patch(id, { folderId });
	}
	async duplicate(id: string) {
		const conv = await call(commands.duplicateConversation(id), src);
		this.put(conv);
		toasts.success("대화 복제함");
		return conv;
	}
	async branch(id: string, messageId: string) {
		const conv = await call(commands.branchConversation(id, messageId), src);
		this.put(conv);
		return conv;
	}
	async remove(id: string) {
		await call(commands.deleteConversation(id), src);
		this.conversations = this.conversations.filter((c) => c.id !== id);
	}
	async setOptions(id: string, o: { personaId: string; promptPresetId: string | null; apiProfileId: string | null }) {
		this.put(await call(commands.setConversationOptions(id, o), src));
	}
	async setImagesEnabled(id: string, enabled: boolean) {
		await call(commands.setConversationImagesEnabled(id, enabled), src);
		this.patch(id, { imagesEnabled: enabled });
	}

	// ---- 폴더 ----
	async loadFolders(charId: string, force = false) {
		if (!force && this.foldersLoaded.has(charId)) return;
		const list = await call(commands.listConversationFolders(charId), src);
		this.folders = [...this.folders.filter((f) => f.characterId !== charId), ...list];
		this.foldersLoaded.add(charId);
	}
	async createFolder(charId: string, name: string) {
		if (!name.trim()) return null;
		const f = await call(commands.createConversationFolder(charId, name.trim()), src);
		this.folders.push(f);
		return f;
	}
	async renameFolder(id: string, name: string) {
		const f = this.folders.find((f) => f.id === id);
		if (!f || !name.trim() || f.name === name.trim()) return;
		await call(commands.renameConversationFolder(id, name.trim()), src);
		f.name = name.trim();
	}
	/** 폴더만 지우고 대화는 폴더 밖으로 */
	async removeFolder(id: string) {
		await call(commands.deleteConversationFolder(id), src);
		this.folders = this.folders.filter((f) => f.id !== id);
		for (const c of this.conversations) if (c.folderId === id) c.folderId = null;
	}

	// ---- 캐릭터 이미지 에셋 ----
	async loadImages(charId: string) {
		this.images[charId] = await call(commands.listCharacterImages(charId), { ...src, silent: true }).catch(() => []);
	}
	async addImages(charId: string, paths: string[]) {
		if (!paths.length) return;
		await call(commands.addCharacterImages(charId, paths), src);
		await this.loadImages(charId);
	}
	async updateImage(img: CharacterImage, p: Partial<Pick<CharacterImage, "key" | "enabled" | "sortOrder">>) {
		const next = { key: img.key, enabled: img.enabled, sortOrder: img.sortOrder, ...p };
		const saved = await call(commands.updateCharacterImage(img.id, next), src);
		const list = this.images[img.characterId] ?? [];
		const i = list.findIndex((x) => x.id === img.id);
		if (i >= 0) list[i] = saved;
	}
	async removeImage(img: CharacterImage) {
		await call(commands.deleteCharacterImage(img.id), src);
		this.images[img.characterId] = (this.images[img.characterId] ?? []).filter((x) => x.id !== img.id);
	}

	// ---- 가져오기 ----
	/** SillyTavern 파일(카드·채팅 등). 채팅이면 `characterId`에 붙인다 */
	async importSillyTavern(path: string, characterId: string | null = null) {
		const r = await call(commands.importSillytavern(path, null, characterId), src);
		await this.load();
		if (r.kind === "character") toasts.success(`'${r.character.name}' 가져옴`);
		else if (r.kind === "chat") toasts.success(`대화 가져옴 · 메시지 ${r.messageCount}개`);
		else toasts.success("가져옴");
		for (const w of r.warnings) toasts.info(w);
		return r;
	}
}

export const library = new Library();
