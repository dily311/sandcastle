// game_state_defs / game_nodes / game_choices 테이블 구조를 그대로 타입으로 옮긴 것입니다.

export type ConditionOp = "gte" | "lte" | "gt" | "lt" | "eq" | "has";

export type Condition = Partial<Record<ConditionOp, Record<string, unknown>>>;

export interface Effect {
	add?: Record<string, number>;
	set?: Record<string, unknown>;
}

export interface LineNode {
	id: string;
	type: "line";
	speaker?: string;
	text: string;
	next: string | null;
}

export interface Choice {
	text: string;
	next: string | null;
	effect?: Effect;
}

export interface ChoiceNode {
	id: string;
	type: "choice";
	choices: Choice[];
}

export interface Branch {
	condition?: Condition;
	next: string | null;
}

export interface BranchNode {
	id: string;
	type: "branch";
	branches: Branch[];
}

export interface AiNode {
	id: string;
	type: "ai";
	characterId: string;
	promptContext?: string; // 첫 호출의 입력이 되는 상황 설명
	maxTurns?: number; // AI가 스스로 이어갈 수 있는 최대 왕복 횟수 (기본 5)
	next: string | null; // AI가 <choice> 없이 응답을 마치면 여기로
}

/** <state>키 += 값</state> 형태로 온 걸 파싱한 결과 */
export interface StateChangeInstruction {
	key: string;
	op: "add" | "sub" | "set";
	value: unknown;
}

export type SceneNode = LineNode | ChoiceNode | BranchNode | AiNode;

export interface StateDef {
	type: "number" | "boolean" | "string" | "array";
	default: unknown;
}

export type StateDefs = Record<string, StateDef>;

export type GameState = Record<string, unknown>;

export interface Scene {
	id: string;
	nodes: SceneNode[];
}

// ---------------------------------------
// command
// ---------------------------------------
export interface GameCommand {
	// 플레이어 이동
	move: {
		from_id: string | null;
		to_id: string;
	};
	// 상호작용
	interact: {
		entity_id: string;
	};
	// 대화
	talk: {
		game_character_id: string;
	};
	// 변수 세팅
	setVariable: {
		key: string;
		value: string;
	};
	useItem: {
		item_id: string;
	};
	// 선택지
	chooseChoice: {
		choice_id: string;
	};
}
