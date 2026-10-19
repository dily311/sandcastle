// npcResponder.send()를 'structured_game' 형식으로 호출하고, 응답을 파싱한 뒤
// conversation/message에 로깅까지 담당합니다.
//
// 1. 캐릭터와 처음 대화가 생성되는 시점에 conversation을 만들고
// 2. 그 conversation_id를 세션 동안 들고 있다가
// 3. 이후 메시지는 그 id로 계속 append하는 방식입니다.
//
// 재생성은 chatStore.ts의 messages.addSwipe/selectSwipe를 그대로 씁니다 — 새
// 메시지 row를 또 만드는 게 아니라 같은 row의 스와이프에 후보를 추가해요. 이래야
// 다음 턴에 AI에게 보낼 대화 기록에 "재도전들"이 실제 대화처럼 섞여 들어가지 않아요.
//
// 주의: conversationId는 메모리에도 캐싱하지만, DB(game_slot_conversations)에도
// 영구히 남겨서 앱을 재시작해도 대화가 이어집니다.

import { conversations, messages } from "$/lib/chat/chatStore.svelte";
import { appSetting } from "$/lib/stores/settingStore.svelte";
import { gameCommands } from "$ipc/commands";
import { get } from "svelte/store";
import type { NpcResponder } from "./npcResponder";
import { parseAiResponse, type ParsedAiResponse } from "./responseParser";
import type { Message, MessageMeta } from "$/lib/type/conv";

export interface GameLogContext {
	gameId: string;
	slotNumber: number;
}

export interface ConversationLookup {
	conversationId: string;
	/** 이미 있던 대화를 찾은 거면 true (새로 만든 게 아니라). 이전 기록을 불러와 보여줄지 판단하는 데 씁니다. */
	isExisting: boolean;
}

export class NpcConversationTracker {
	private conversationIds = new Map<string, string>(); // characterId -> conversationId

	constructor(private readonly ctx: GameLogContext) {}

	async getOrCreate(characterId: string): Promise<ConversationLookup> {
		const cached = this.conversationIds.get(characterId);
		if (cached) {
			messages.loadForConversation(cached);
			return { conversationId: cached, isExisting: true };
		}

		// 메모리에 없으면 DB에 이미 만들어둔 게 있는지 먼저 확인합니다.
		// (재시작해도 여기서 다시 찾아지니, 대화가 안 끊깁니다.)
		const existingId = await gameCommands.getSlotConversationId({
			gameId: this.ctx.gameId,
			slotNumber: this.ctx.slotNumber,
			characterId,
		});

		if (existingId) {
			this.conversationIds.set(characterId, existingId);
			messages.loadForConversation(existingId);
			return { conversationId: existingId, isExisting: true };
		}

		const personaId = appSetting.selectedPersonaId;
		const conversation = await conversations.create({
			title: "",
			characterId,
			personaId,
			worldInfoIds: [],
			mode: "game",
			meta: { gameId: this.ctx.gameId, slotNumber: this.ctx.slotNumber },
		});

		await gameCommands.linkSlotConversation({
			input: {
				gameId: this.ctx.gameId,
				slotNumber: this.ctx.slotNumber,
				characterId,
				conversationId: conversation.id,
			}
		});

		this.conversationIds.set(characterId, conversation.id);
		messages.loadForConversation(conversation.id);
		return { conversationId: conversation.id, isExisting: false };
	}
}

export interface NpcTurnResult {
	speakerName: string;
	parsed: ParsedAiResponse;
	/** 이 턴에서 만들어진(혹은 갱신된) 메시지 row입니다. 재생성/스와이프 전환에 씁니다. */
	message: Message;
	history: Message[];
}

function metaFor(nodeId: string, res: { text: string }, parsed: ParsedAiResponse): MessageMeta {
	return { nodeId, rawText: res.text, stateChanges: parsed.stateChanges } as MessageMeta;
}

/** AI 노드가 한 턴 진행할 때마다 호출합니다 (그 턴의 첫 시도). */
export async function callNpcStructured(
	responder: NpcResponder,
	tracker: NpcConversationTracker,
	nodeId: string,
	args: { characterId: string; userMessage: string; extraContext?: string },
): Promise<NpcTurnResult> {
	const res = await responder.send({ ...args, responseFormat: "structured_game" });
	const parsed = parseAiResponse(res.text);

	let history: Message[] = get(messages);
	let message: Message = {
		id: crypto.randomUUID(),
		conversationId: "",
		role: "assistant",
		name: res.speakerName,
		content: parsed.cleanedText,
		characterId: args.characterId,
		meta: metaFor(nodeId, res, parsed),
		createdAt: Date.now(),
	};

	try {
		const conversationId = await tracker.getOrCreate(args.characterId);
		const convId: string = conversationId.conversationId;
		if (args.userMessage) {
			await messages.append({
				conversationId: convId,
				role: "user",
				name: "플레이어",
				content: args.userMessage,
				characterId: undefined,
				meta: { nodeId },
			});
		}

		message = await messages.append({
			conversationId: convId,
			role: "assistant",
			name: res.speakerName,
			content: parsed.cleanedText,
			characterId: args.characterId,
			meta: metaFor(nodeId, res, parsed),
		});

	} catch (err) {
		console.warn("[npc log] 대화 저장에 실패했지만 게임 진행에는 영향 없어요.", err);
	}

	return { speakerName: res.speakerName, parsed, message, history };
}

/**
 * 재생성: 같은 조건으로 다시 물어봐서 결과를 messages.addSwipe로 추가합니다.
 * 새 row를 만들지 않고, 기존 message의 content/meta.swipeId/meta.swipes만 갱신돼요.
 */
export async function addSwipe(
	currentMessage: Message,
	responder: NpcResponder,
	nodeId: string,
	args: { characterId: string; userMessage: string; extraContext?: string },
): Promise<NpcTurnResult> {
	const res = await responder.send({ ...args, responseFormat: "structured_game" });
	const parsed = parseAiResponse(res.text);

	let message = currentMessage;
	try {
		// 주의: messages.addSwipe의 changedMeta 파라미터가 타입상 MessageSwipe로
		// 돼있는데, 실제 구현은 이걸 message.meta에 병합하는 용도로 쓰고 있어서
		// (content/createdAt 없이) 여기서는 메타 필드만 담아 넘깁니다. 타입 에러가
		//나면 changedMeta를 Partial<MessageMeta>로 완화해주시는 게 맞을 것 같아요.
		const updated = await messages.addSwipe(
			currentMessage.id,
			parsed.cleanedText,
			metaFor(nodeId, res, parsed) as never,
		);
		if (updated) message = updated;
	} catch (err) {
		console.warn("[npc log] 재생성 결과 저장에 실패했지만 화면엔 반영돼요.", err);
	}

	return { speakerName: res.speakerName, parsed, message };
}

/** AI 호출 없이, 이미 저장된 스와이프 중 하나로 전환합니다 (이전/다음 변형 보기). */
export async function selectSwipe(currentMessage: Message, index: number): Promise<Message> {
	try {
		const updated = await messages.selectSwipe(currentMessage.id, index);
		if (updated) return updated;
	} catch (err) {
		console.warn("[npc log] 스와이프 전환 저장에 실패했지만 화면엔 반영돼요.", err);
	}
	return currentMessage;
}
