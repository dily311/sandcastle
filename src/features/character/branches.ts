// 분기 경로: 대화 meta.branch = { parentId, messageId, seq } 를 따라 위(조상)·아래(갈래)를 찾는다
import { library, type Conversation } from "./library.svelte";

export interface BranchPoint {
	parentId: string;
	messageId: string;
	seq: number;
}

export function branchOf(c: Conversation | undefined | null): BranchPoint | null {
	const b = (c?.meta as { branch?: BranchPoint } | null)?.branch;
	return b && typeof b.parentId === "string" ? b : null;
}

/** 이 대화에서 갈라져 나간 대화들 (갈라진 메시지 순, 같으면 만든 순) */
export function childrenOf(id: string): Conversation[] {
	return library.conversations
		.filter((c) => branchOf(c)?.parentId === id)
		.sort((a, b) => (branchOf(a)?.seq ?? 0) - (branchOf(b)?.seq ?? 0) || a.createdAt - b.createdAt);
}

export interface PathStep {
	/** 지운 대화면 없음 */
	conv?: Conversation;
	/** 다음 단계가 이 대화의 몇 번째 메시지에서 갈라졌는지 */
	branchAt?: BranchPoint;
}

/** 맨 위 조상부터 이 대화까지. 부모가 지워졌으면 맨 앞에 빈 단계 */
export function pathOf(conv: Conversation): PathStep[] {
	const steps: PathStep[] = [{ conv }];
	const seen = new Set([conv.id]);
	let cur: Conversation = conv;
	for (let i = 0; i < 50; i++) {
		const b = branchOf(cur);
		if (!b) break;
		const parent = library.conversation(b.parentId);
		if (!parent || seen.has(parent.id)) {
			steps.unshift({ branchAt: b });
			break;
		}
		seen.add(parent.id);
		steps.unshift({ conv: parent, branchAt: b });
		cur = parent;
	}
	return steps;
}

export interface BranchNode {
	conv: Conversation;
	/** 부모에서 갈라진 지점 (맨 위는 없음) */
	at: BranchPoint | null;
	children: BranchNode[];
}

/** 이 대화가 속한 갈래 전체 (맨 위 조상부터) */
export function familyOf(conv: Conversation): BranchNode {
	const top = pathOf(conv).find((s) => s.conv)?.conv ?? conv;
	const seen = new Set<string>();
	const build = (c: Conversation): BranchNode => {
		seen.add(c.id);
		return { conv: c, at: c.id === top.id ? null : branchOf(c), children: childrenOf(c.id).filter((x) => !seen.has(x.id)).map(build) };
	};
	return build(top);
}

/** 갈래 안의 대화 수 (자기 포함) */
export function familySize(node: BranchNode): number {
	return 1 + node.children.reduce((n, c) => n + familySize(c), 0);
}
