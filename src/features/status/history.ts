// 지난 시점의 대화 변수 (5-E-6-1): 지금 값에서, 그 메시지 뒤 메시지들이 AI로 바꾼 몫(meta.varChanges)을 거꾸로 되돌린다.
// 사용자가 손으로 바꾼 값은 기록이 없어 되돌리지 못한다 (지금 값 그대로)

type Change = { key: string; from: string | null; to: string };
type Msg = { meta?: unknown };

function changesOf(m: Msg): Change[] {
	const c = (m.meta as { varChanges?: unknown } | null | undefined)?.varChanges;
	return Array.isArray(c) ? (c.filter((x) => x && typeof (x as Change).key === "string") as Change[]) : [];
}

/** index번 메시지까지 읽었을 때의 변수 (index 뒤 메시지들의 바뀜을 뒤에서부터 되돌림) */
export function varsAt(current: Record<string, unknown>, messages: readonly Msg[], index: number): Record<string, unknown> {
	const vars: Record<string, unknown> = { ...current };
	for (let i = messages.length - 1; i > index; i--) {
		const changes = changesOf(messages[i]);
		for (let k = changes.length - 1; k >= 0; k--) {
			const c = changes[k];
			if (c.from === null || c.from === undefined) delete vars[c.key];
			else vars[c.key] = c.from;
		}
	}
	return vars;
}
