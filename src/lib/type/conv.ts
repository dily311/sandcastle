
export interface Persona {
    id: string,
    name: string,
    avatar: string | null,
    description: string,
    // TODO: 추후 추가 예정 (구현하기 복잡할 듯)
    // 그냥 여러 id 페르소나를 두고 한 페르소나에서 보여주는 그룹 형태로 구현하는 게 나을 지도
    // alternateDescriptions: string[] | null;
    worldInfoIds: string[],
    tags: string[],
    color: string,
    isSystemDefault: boolean,
    createdAt: number,
    updatedAt: number,
}
export type PersonaInput = Omit<Persona, "id" | "createdAt" | "updatedAt">;

export interface Character {
    id: string,
    name: string,
    avatar: string | null,
    description: string,
    alternateDescriptions: string[] | null;
    firstMessage: string | null,
    alternateFirstMessage: string[] | null;
    exampleDialogue: string | null,
    systemPromptOverride: string | null,
    worldInfoIds: string[],
    color: string | null,
    tags: string[],
    category: string | null,
    defaultPersonaId: string | null,
    favorite: boolean,
    messageCount: number,
    createdAt: number,
    updatedAt: number,
}

export type CharacterInput = Omit<Character, "id" | "createdAt" | "updatedAt" | "messageCount">;

export interface WorldInfoBook {
    id: string,
    name: string,
    description: string | null,
    createdAt: number,
    updatedAt: number,
}

export interface WorldInfoEntry {
    id: string,
    bookId: string,
    keys: string[],
    secondaryKeys: string[],
    content: string,
    enabled: boolean,
    priority: number,
    constant: boolean,
}

export type conversationMode = "chat" | "game";

export interface Conversation {
    id: string,
    title: string,
    characterId: string,
    personaId: string,
    worldInfoIds: string[],
    mode: string,
    meta: ConversationMeta | null,
    pinned: boolean,
    createdAt: number,
    updatedAt: number,
    messageCount: number,
}

export interface ConversationMeta {
	worldInfoIds?: string[];
	// 분기 정보
	mainChatId?: string;
	// 해당 대화에서만 주입될 프롬프트
	notePromptInfo?: {
		prompt: string;
		// 깊이
		depth: number;
		// 위치
		position: number;
		// 규칙
		role: number;
	};
	[key: string]: unknown;
}

export interface Message {
    id: string,
    conversationId: string,
    role: string,
    name: string | null,
    content: string,
    characterId: string | null,
    meta: MessageMeta | null,
    createdAt: number,
    // 프론트에서만 사용
    status?: string,
}

export type MessageSwipe = Omit<Message, "id" | "name" | "role" | "status" | "conversationId" | "characterId">;
export interface MessageMeta {
	// 보낸 정보 기록
	// 나중에 캐릭터나 페르소나를 수정하게 되더라도 수정 전 이름으로 표시되게 하기 위함
	// 캐릭터 이름 수정 시 sender.name을 일괄 수정할 수 있게도 할 예정
	sender?: {
		type: "system" | "user" | "assistant";
		id?: string;
		name: string;
		avatar: string | null;
		color: string | null;
	};
	// api 정보
	info?: {
		api?: string;
		model?: string;
		presetId?: string;
		tokenUsage?: {
			prompt?: number;
			completion?: number;
		};
		// 스트리밍시 시작시간과 종료시간 기록
		genStarted?: number;
		genFinished?: number;
	};
	// 스와이프
	// swipeId가 현재 표시되는 메시지
	swipeId?: number;
	swipes?: MessageSwipe[];
	[key: string]: unknown;
}
