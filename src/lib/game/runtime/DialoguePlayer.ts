import type {
	Scene,
	StateDefs,
	GameState,
	SceneNode,
	LineNode,
	ChoiceNode,
	AiNode,
	Condition,
	Effect,
	StateChangeInstruction,
} from "../types";

// 씬 JSON을 순회하며 상태(state)를 읽고 쓰는 순수 로직입니다.
// pixi.js, DOM, Svelte 어느 것에도 의존하지 않아서
// 2단계(iframe)에서도 이 파일을 그대로 iframe 안에 넣기만 하면 됩니다.

export class DialoguePlayer {
	private scene: Scene;
	private nodesById: Map<string, SceneNode>;
	private currentId: string | null;

	state: GameState;

	constructor(scene: Scene, stateDefs: StateDefs) {
		this.scene = scene;
		this.nodesById = new Map(scene.nodes.map((n) => [n.id, n]));

		this.state = {};
		for (const [key, def] of Object.entries(stateDefs)) {
			this.state[key] = def.default;
		}

		this.currentId = scene.nodes[0]?.id ?? null;
		this.skipBranches();
	}

	/** 세이브 데이터에서 불러온 nodeId/state로 진행 상태를 되돌립니다. */
	restore(nodeId: string, state: GameState): void {
		this.currentId = nodeId;
		this.state = { ...state };
	}

	/** 세이브할 때 저장해야 할 "현재 위치" (항상 line/choice 노드 id, branch는 저장 안 됨) */
	getCurrentId(): string | null {
		return this.currentId;
	}

	/** AI 응답의 <state> 태그를 파싱한 결과를 적용합니다. */
	applyStateChanges(changes: StateChangeInstruction[]): void {
		for (const change of changes) {
			if (change.op === "add") {
				this.state[change.key] = ((this.state[change.key] as number) ?? 0) + (change.value as number);
			} else if (change.op === "sub") {
				this.state[change.key] = ((this.state[change.key] as number) ?? 0) - (change.value as number);
			} else {
				this.state[change.key] = change.value;
			}
		}
	}

	/** 현재 화면에 보여줘야 할 노드 (line/choice/ai), 없으면 null(=씬 종료) */
	current(): LineNode | ChoiceNode | AiNode | null {
		if (!this.currentId) return null;
		const node = this.nodesById.get(this.currentId);
		if (!node || node.type === "branch") return null; // branch는 skipBranches가 항상 처리해줌
		return node;
	}

	/** line/ai 노드에서 "다음으로" 넘어갈 때 (ai 노드는 응답을 다 보여준 뒤 호출) */
	advance(): void {
		const node = this.current();
		if (!node || (node.type !== "line" && node.type !== "ai")) return;
		this.currentId = node.next;
		this.skipBranches();
	}

	/** choice 노드에서 선택지를 골랐을 때 */
	choose(choiceIndex: number): void {
		const node = this.current();
		if (!node || node.type !== "choice") return;
		const choice = node.choices[choiceIndex];
		if (!choice) return;

		if (choice.effect) this.applyEffect(choice.effect);
		this.currentId = choice.next;
		this.skipBranches();
	}

	/** branch 노드는 유저 입력 없이 조건만 평가하고 자동으로 통과시킵니다 */
	private skipBranches(): void {
		let guard = 0; // 무한루프 방지
		while (this.currentId && guard++ < 100) {
			const node = this.nodesById.get(this.currentId);
			if (!node || node.type !== "branch") break;

			const matched = node.branches.find((b) => !b.condition || this.evalCondition(b.condition));
			this.currentId = matched?.next ?? null;
		}
	}

	private evalCondition(cond: Condition): boolean {
		const entry = Object.entries(cond)[0];
		if (!entry) return false;
		const [op, checks] = entry;
		const checkEntry = Object.entries(checks)[0];
		if (!checkEntry) return false;
		const [key, value] = checkEntry;
		const current = this.state[key];

		switch (op) {
			case "gte":
				return (current as number) >= (value as number);
			case "lte":
				return (current as number) <= (value as number);
			case "gt":
				return (current as number) > (value as number);
			case "lt":
				return (current as number) < (value as number);
			case "eq":
				return current === value;
			case "has":
				return Boolean(current);
			default:
				return false;
		}
	}

	private applyEffect(effect: Effect): void {
		if (effect.add) {
			for (const [key, value] of Object.entries(effect.add)) {
				this.state[key] = ((this.state[key] as number) ?? 0) + value;
			}
		}
		if (effect.set) {
			for (const [key, value] of Object.entries(effect.set)) {
				this.state[key] = value;
			}
		}
	}
}
