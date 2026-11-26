// 캐릭터·대화·폴더와 대화 설정에 쓰는 목록(페르소나·프리셋·API). 화면들이 같이 쓴다.
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { localizeImportWarning } from "$lib/utils/backendI18n";
import { jobs } from "$shared/stores/jobs.svelte";
import { trashedToast } from "$shared/trash";
import type {
	ApiProfiles,
	CharacterImage_Serialize as CharacterImage,
	Character_Serialize as Character,
	ConversationFolder,
	Conversation_Serialize as Conversation,
	Persona_Serialize as Persona,
	PromptPresets_Serialize as Preset,
	VarDef_Serialize as VarDef,
} from "$ipc/bindings";
import { call, commands } from "$shared/ipc/client";
import { toasts } from "$shared/stores/toast.svelte";
import { logs } from "$shared/stores/log.svelte";

const tr = (key: string, values?: Record<string, string | number>) => get(_)(key, values ? { values } : undefined);

export type { Character, CharacterImage, Conversation, ConversationFolder, Persona, Preset, ApiProfiles, VarDef };

/** 목록에 보일 대화 시각: 마지막 메시지 시각 (가져온 대화도 원래 대화 시각으로). 메시지가 없으면 수정 시각 */
// ---- 목록 캐시 (이 기기 브라우저 저장소, 보여 주기만 — 곧 새로 읽은 값으로 바뀐다) ----
const LIST_CACHE = "sc.library.list";
const LIST_CACHE_MAX = 2_000_000;
function readListCache(): { characters: Character[]; conversations: Conversation[] } | null {
	try {
		const v = JSON.parse(localStorage.getItem(LIST_CACHE) ?? "null");
		return v && Array.isArray(v.characters) && Array.isArray(v.conversations) ? v : null;
	} catch {
		return null;
	}
}
function writeListCache(characters: Character[], conversations: Conversation[]) {
	try {
		// 목록에 필요 없는 큰 칸(대화 meta·캐릭터 설명)은 빼고, 그래도 크면 남기지 않는다
		const slim = {
			characters: characters.map((c) => ({ ...c, description: "", exampleDialogue: null, firstMessage: null, alternateDescriptions: [], alternateFirstMessage: [] })),
			conversations: conversations.map((c) => ({ ...c, meta: null })),
		};
		const text = JSON.stringify(slim);
		if (text.length > LIST_CACHE_MAX) localStorage.removeItem(LIST_CACHE);
		else localStorage.setItem(LIST_CACHE, text);
	} catch {
		/* 저장소를 못 쓰면 캐시 없이 */
	}
}

export const lastAt = (c: Pick<Conversation, "lastMessageAt" | "updatedAt">) => c.lastMessageAt ?? c.updatedAt;

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
		// 처음 열 때: 지난번 목록을 먼저 보여 주고 뒤에서 새로 읽는다 (대화가 많으면 읽는 데 시간이 걸림, 5-E-8)
		if (!this.loaded) {
			const cached = readListCache();
			if (cached) {
				Object.assign(this, cached);
				this.loaded = true;
			}
		}
		const [characters, conversations, personas, presets, apis, conv] = await Promise.all([
			call(commands.getAllCharacters(), src),
			call(commands.getAllConversations(), src),
			call(commands.getAllPersonas(), { ...src, silent: true }).catch(() => []),
			call(commands.listPromptPresets(), { ...src, silent: true }).catch(() => []),
			call(commands.getAllApiProfile(), { ...src, silent: true }).catch(() => []),
			call(commands.getConvSettings(), { ...src, silent: true }).catch(() => null),
		]);
		Object.assign(this, { characters, conversations, personas, presets, apis });
		this.defaultPersonaId = conv?.defaultPersonaId ?? null;
		this.loaded = true;
		writeListCache(characters, conversations);
	}

	/** API 프로필 목록만 최신 상태(연결 확인 여부 등)로 다시 읽는다 */
	async refreshApis() {
		const apis = await call(commands.getAllApiProfile(), { ...src, silent: true }).catch(() => null);
		if (apis) this.apis = apis;
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
	/** 전역 기본 페르소나 (사이드바에서 고름, ADR-028) */
	defaultPersonaId = $state<string | null>(null);
	/** 대화가 쓰는 페르소나: 대화에 있으면 그것(자물쇠) → 캐릭터 기본 → 전역. 백엔드 resolve_conv_persona와 같은 순서 */
	convPersonaId(c: Pick<Conversation, "personaId" | "characterId"> | null | undefined): string {
		return c?.personaId || this.character(c?.characterId)?.defaultPersonaId || this.defaultPersonaId || "default";
	}
	/** 사이드바에서 유저 프로필 고르기 = 언제나 전역 (자물쇠를 건 대화는 그대로) */
	async setDefaultPersona(id: string) {
		await call(commands.setSystemDefaultPersona(id), src);
		this.defaultPersonaId = id;
	}
	/** 최근 활동 순 (폰 대화는 폰 화면에서만 보임) */
	convsOf(charId: string) {
		return this.conversations.filter((c) => c.characterId === charId && c.mode !== "phone").sort((a, b) => lastAt(b) - lastAt(a));
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
		if (!ch) throw new Error(tr("chatExtra.characterNotFound"));
		const n = this.convsOf(charId).length + 1;
		// 페르소나를 복사하지 않는다: 비워 두면 캐릭터 기본 → 전역을 따른다 (ADR-028). 개요 탭 자물쇠로 이 대화에 고정
		const conv = await call(
			commands.createConversation({ title: tr("chatExtra.newChatTitle", { name: ch.name, n }), characterId: charId, personaId: "", worldInfoIds: [], mode: "chat" }),
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
			commands.createConversation({ title: ch?.name ?? tr("chatExtra.phoneChatDefaultTitle"), characterId: charId, personaId, worldInfoIds: [], mode: "phone" }),
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
	async setReadMode(id: string, on: boolean) {
		await call(commands.setConversationReadMode(id, on), src);
		this.patch(id, { readMode: on });
	}
	async move(id: string, folderId: string | null) {
		await call(commands.moveConversation(id, folderId), src);
		this.patch(id, { folderId });
	}
	async duplicate(id: string) {
		const conv = await call(commands.duplicateConversation(id), src);
		this.put(conv);
		toasts.success(tr("chatExtra.duplicatedToast"));
		return conv;
	}
	async branch(id: string, messageId: string) {
		const conv = await call(commands.branchConversation(id, messageId), src);
		this.put(conv);
		return conv;
	}
	/** 휴지통으로 (2-A-3). "되돌리기" 알림을 띄운다 */
	async remove(id: string) {
		const item = await jobs.run(this.conversation(id)?.title || tr("chatExtra.convFallback"), () => call(commands.deleteConversation(id), src), { group: tr("chatExtra.deleteConvGroup") });
		if (!item) return;
		this.conversations = this.conversations.filter((c) => c.id !== id);
		trashedToast(item, () => this.load());
	}
	async setOptions(id: string, o: { personaId: string; promptPresetId: string | null; apiProfileId: string | null; model?: string | null }) {
		this.put(await call(commands.setConversationOptions(id, { ...o, model: o.model ?? null }), src));
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
		const name = this.folders.find((f) => f.id === id)?.name ?? tr("chatExtra.folderFallback");
		if ((await jobs.run(name, () => call(commands.deleteConversationFolder(id), src), { group: tr("chatExtra.deleteFolderGroup") })) === undefined) return;
		this.folders = this.folders.filter((f) => f.id !== id);
		for (const c of this.conversations) if (c.folderId === id) c.folderId = null;
	}

	// ---- 변수 정의 (5-F-4-3, ADR-029): 캐릭터마다. 상태창·변수 플러그인이 켜졌을 때만 읽는다 ----
	varDefs = $state<Record<string, VarDef[]>>({});
	async loadVarDefs(charId: string) {
		this.varDefs[charId] = await call(commands.getCharacterVarDefs(charId), { ...src, silent: true }).catch(() => []);
		return this.varDefs[charId];
	}
	async setVarDefs(charId: string, defs: VarDef[]) {
		this.varDefs[charId] = await call(commands.setCharacterVarDefs(charId, defs), src);
		return this.varDefs[charId];
	}

	/** 캐릭터의 상태창 틀 (5-F-4-5). 없으면 null */
	statusTemplates = $state<Record<string, string | null>>({});
	async loadStatusTemplate(charId: string) {
		this.statusTemplates[charId] = await call(commands.getCharacterStatusTemplate(charId), { ...src, silent: true }).catch(() => null);
		return this.statusTemplates[charId];
	}
	async setStatusTemplate(charId: string, template: string | null) {
		await call(commands.setCharacterStatusTemplate(charId, template?.trim() ? template : null), src);
		this.statusTemplates[charId] = template?.trim() ? template : null;
	}

	// ---- 캐릭터 에셋 에셋 ----
	/** 캐릭터가 쓸 에셋 판 (null = 전체, 5-E-21) */
	assetVersion = $state<Record<string, string | null>>({});
	async loadImages(charId: string) {
		const [imgs, version] = await Promise.all([
			call(commands.listCharacterImages(charId), { ...src, silent: true }).catch(() => []),
			call(commands.getCharacterAssetVersion(charId), { ...src, silent: true }).catch(() => null),
		]);
		this.images[charId] = imgs;
		this.assetVersion[charId] = version;
	}
	async setAssetVersion(charId: string, version: string | null) {
		await call(commands.setCharacterAssetVersion(charId, version), src);
		this.assetVersion[charId] = version;
	}
	/** 대화에 쓰는 그림: 고른 판과 공통(폴더 없음). 판을 고르지 않으면 전체 */
	imagesInUse(charId: string) {
		const v = this.assetVersion[charId];
		return (this.images[charId] ?? []).filter((i) => !v || !i.folder || i.folder.toLowerCase() === v.toLowerCase());
	}
	async addImages(charId: string, paths: string[]) {
		if (!paths.length) return;
		await jobs.run(tr("chatExtra.imagesCount", { count: paths.length }), () => call(commands.addCharacterImages(charId, paths), src), { group: tr("chatExtra.uploadAssetsGroup") });
		await this.loadImages(charId);
	}
	async updateImage(img: CharacterImage, p: Partial<Pick<CharacterImage, "key" | "enabled" | "sortOrder" | "folder">>) {
		const next = { key: img.key, enabled: img.enabled, sortOrder: img.sortOrder, folder: img.folder, ...p };
		const saved = await call(commands.updateCharacterImage(img.id, next), src);
		const list = this.images[img.characterId] ?? [];
		const i = list.findIndex((x) => x.id === img.id);
		if (i >= 0) list[i] = saved;
	}
	/** ZIP 안의 그림을 한꺼번에 (파일 이름이 키, 같은 키는 덮어씀) */
	async addImagesZip(charId: string, zipPath: string) {
		const added = await jobs.run(zipPath.split(/[\\/]/).pop() ?? "ZIP", () => call(commands.addCharacterImagesZip(charId, zipPath), src), { group: tr("chatExtra.uploadAssetsGroup") });
		await this.loadImages(charId);
		return added?.length ?? 0;
	}
	async removeImage(img: CharacterImage) {
		if ((await jobs.run(img.key || tr("chatExtra.imageFallback"), () => call(commands.deleteCharacterImage(img.id), src), { group: tr("chatExtra.deleteAssetsGroup") })) === undefined) return;
		this.images[img.characterId] = (this.images[img.characterId] ?? []).filter((x) => x.id !== img.id);
	}

	// ---- 가져오기 ----
	/** SillyTavern 파일(카드·채팅 등). 채팅이면 `characterId`에 붙인다 */
	/** `quiet`: 여러 파일을 차례로 가져올 때 — 파일마다 목록을 다시 읽거나 알리지 않는다 (경고는 로그에만) */
	async importSillyTavern(path: string, characterId: string | null = null, opts: { quiet?: boolean } = {}) {
		const r = await call(commands.importSillytavern(path, null, characterId, null), src);
		const warnings = r.warnings.map(localizeImportWarning);
		if (opts.quiet) {
			for (const w of warnings) logs.add("warn", "import", w);
			return r;
		}
		await this.load();
		if (r.kind === "character") toasts.success(tr("chatExtra.charImportedToast", { name: r.character.name }));
		else if (r.kind === "chat") toasts.success(tr("chatExtra.chatImportedToast", { count: r.messageCount }));
		else toasts.success(tr("chatExtra.importedToast"));
		for (const w of warnings) toasts.info(w);
		return r;
	}
}

export const library = new Library();
