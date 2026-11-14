import type { Character, Conversation, ConversationInput, Message, MessageInput, MessageMeta, MessageSwipe } from "$/lib/type/conv";
import { appSetting } from "../stores/settingStore.svelte";
import { dbCommands } from "$/ipc/commands";
import { apiProfiles } from "../stores/apiProfileStore.svelte";
import { characters } from "./characterStore.svelte";

// -----------------------------------
// conversations
// -----------------------------------
class ConversationStore {
	#store = $state<Conversation[]>([]);
	activeId = $state<string | null>(null);
	isChattingLoading = $state<boolean>(false);

	get value() {
		return this.#store;
	}
	get activeConv() {
		return this.#store.find((c) => c.id === this.activeId) ?? null;
	}
	get activeCharacter() {
		return this.activeConv ? (characters.value.find((c) => c.id === this.activeConv?.characterId) ?? null) : null;
	}
	get messageCount() {
		const count = (this.activeConv?.messageCount ?? 0) - messages.value.length;
		if (count < 0) return 0;
		return count;
	}

	async loadAll() {
		this.#store = await dbCommands.getAllConversations();
	}

	async touch(id: string) {
		const now = await dbCommands.touchConversation({ id });
		this.#store = this.#store
			.map((c) => (c.id === id ? { ...c, updatedAt: now, messageCount: c.messageCount + 1 } : c))
			.sort((a, b) => b.updatedAt - a.updatedAt);
	}

	async create(input: ConversationInput) {
		const conv = await dbCommands.createConversation({ input });

		if (conv?.characterId) {
			await characters.updateMessageCount(conv.characterId, 1);
		}

		this.#store.push(conv);
		return conv;
	}

	async remove(id: string) {
		await dbCommands.deleteConversation({ id });

		const current = this.#store.find((c) => c.id === id);
		if (current?.characterId) {
			await characters.updateMessageCount(current.characterId, -1);
		}

		this.#store = this.#store.filter((c) => c.id !== id);
	}
}

export const conversations = new ConversationStore();

// -----------------------------------
// --- messages (현재 대화만 메모리에 유지) ---
// -----------------------------------
export const LIMIT_MESSAGE = 50;
class MessageStore {
	private messages = $state<Message[]>([]);
	currentConvId: string | null = null;
	delta: string | null = null;

	get value() {
		return this.messages;
	}
	get activeLoadingMessageCount() {
		return this.messages.filter((msg) => msg.status && (msg.status === "loading" || msg.status === "streaming"))
			.length;
	}

	async setCurrentConvId(convId: string) {
		this.currentConvId = convId;
		this.messages = [];
	}
	async loadForConversation(conversationId: string, opts?: { limit?: number; before?: number }) {
		const msgs = await dbCommands.getMessages(conversationId, opts?.limit ?? LIMIT_MESSAGE, opts?.before);
		if (opts?.before) {
			this.messages.unshift(...msgs); // 위로 스크롤 시 과거 메시지 앞에 붙이기
		} else {
			this.currentConvId = conversationId;
			this.messages = msgs;
		}
	}

	async append(msg: MessageInput) {
		const full = await dbCommands.appendMessage({ input: msg });
		if (full.conversationId === this.currentConvId) this.messages.push(full);
		await conversations.touch(full.conversationId);
		return full;
	}

	/** AI 응답 스트리밍: 빈 메시지 생성 -> delta로 계속 채우기 -> 완료 시 DB flush */
	async startAssistantStream(conversationId: string, character: Character) {
		const message = await this.append({
			conversationId,
			seq: this.messages.length + 1,
			status: "streaming",
			role: "assistant",
			content: "",
			name: character.name,
			characterId: character.id,
			meta: {
				sender: {
					name: character.name,
					type: "assistant",
					avatar: character.avatar,
					color: character.color,
					id: character.id,
				},
				info: {
					genStarted: Date.now(),
				},
			},
		});

		sendToModel({
			conversationId: conversationId,
			character: character,
			onDelta: (delta) => this.appendDelta(message.id, delta),
			onDone: () => this.finalize(message.id),
		});
		return;
	}

	/**  스트리밍 중엔 DB에 업로드 X */
	appendDelta(id: string, delta: string) {
		this.messages = this.messages.map((m) => (m.id === id ? { ...m, content: m.content + delta } : m));
	}

	/** 스트리밍 종료. DB 업데이트 */
	async finalize(id: string) {
		const msg = this.messages.find((m) => m.id === id);
		if (!msg) return;
		msg.status = undefined;
		const meta = {
			...msg.meta,
			info: {
				...msg.meta?.info,
				genFinished: Date.now(),
			},
		};
		await dbCommands.updateMessage({ id, content: msg.content, meta });
		this.messages = this.messages.map((m) => (m.id === id ? { ...m, status: undefined, meta } : m));
	}

	/** 스와이프 추가 */
	async addSwipe(id: string, content: string, changedMeta?: Partial<MessageMeta>): Promise<Message | undefined> {
		const current = this.messages.find((m) => m.id === id);
		if (!current) return;

		const prevMeta = current.meta ?? {};
		const { swipes, swipeId, ...metaWithoutSwipes } = prevMeta;
		const existingSwipes = current.meta?.swipes ?? [];

		const newSwipes: MessageSwipe[] = [...existingSwipes];
		if (existingSwipes.length === 0) {
			const prevMessage: MessageSwipe = {
				content: current.content,
				meta: metaWithoutSwipes ?? {},
				createdAt: current.createdAt,
				updatedAt: current.updatedAt,
			};
			newSwipes.push(prevMessage);
		}
		const swipeMessage: MessageSwipe = {
			content: content,
			meta: {},
			createdAt: Date.now(),
			updatedAt: Date.now(),
		};
		newSwipes.push(swipeMessage);

		const newMessage: Message = {
			...current,
			content: content,
			meta: {
				...current.meta,
				...changedMeta,
				swipes: newSwipes,
				swipeId: newSwipes.length - 1,
			},
		};
		await dbCommands.updateMessage({ id, content, meta: newMessage.meta });
		this.messages = this.messages.map((m) => (m.id === id ? newMessage : m));
		return newMessage;
	}

	/** 스와이프 선택 */
	async selectSwipe(id: string, index: number) {
		const current = this.messages.find((m) => m.id === id);
		if (!current || !(current.meta && current.meta.swipes)) return;

		const selectedSwipe = current.meta.swipes[index];
		const newMessage: Message = {
			...current,
			content: selectedSwipe.content ?? "",
			meta: {
				...current.meta,
				...selectedSwipe.meta,
				swipeId: index,
			},
		};

		await dbCommands.updateMessage({ id, content: selectedSwipe.content, meta: newMessage.meta });
		this.messages = this.messages.map((m) => (m.id === id ? newMessage : m));
		return newMessage;
	}

	async remove(id: string) {
		await dbCommands.deleteMessage({ id });
		this.messages = this.messages.filter((m) => m.id !== id);
	}
}

export const messages = new MessageStore();

// -----------------------------------
// 사용함수
// -----------------------------------
export async function openConversation(id: string) {
	conversations.activeId = id;
	await messages.loadForConversation(id);
}

export async function deleteConversation(convID: string) {
	// 연관된 메시지도 제거
	await conversations.remove(convID);
}

/** 새 대화 세션 생성 */
export async function newConversation(characterId: string) {
	const char = characters.getChar(characterId);
	const personaId = appSetting.selectedPersonaId;

	const conv = await conversations.create({
		title: `${char?.name}와(과) 대화`,
		characterId,
		personaId,
		promptPresetId: null,
		worldInfoIds: [],
		mode: "chat",
		meta: null,
	});
	await openConversation(conv.id);
	if (char && char.firstMessage) {
		await messages.append({
			conversationId: conv.id,
			seq: messages.value.length + 1,
			content: char.firstMessage,
			name: char.name,
			role: "system",
			meta: {
				sender: {
					type: "system",
					id: char.id,
					name: char.name,
					avatar: char.avatar,
					color: char.color,
				},
			},
			characterId: char.id,
		});
	}
	return conv.id;
}

interface SendData {
	conversationId: string;
	character: Character | null;
	onDelta: (delta: any) => void;
	onDone: () => void;
}
/** AI 연결
 * TODO: 목킹만 구현했으므로 실제 연결 단계 구현해야 함
 */
export async function sendToModel(data: SendData) {
	// API 연결 확인
	if (!apiProfiles.isConnected) {
		conversations.isChattingLoading = true;
		// 더미
		setTimeout(async () => {
			data.onDelta("뭐라고?");

			// 연결 이후 로딩
			conversations.isChattingLoading = false;
			data.onDone();
		}, 800);
	} else {
		// 연결되지 않았을 경우 오류 메시지 출력
		return Error("API에 연결되지 않았습니다.");
	}
}
