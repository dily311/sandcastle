// 메시지 linter 결과 (3-B, ADR-014). 백엔드가 응답을 저장할 때 meta.lint에 남긴다
import type { Message_Serialize as Message } from "$ipc/bindings";

export type LintFinding = { rule: string; level: "info" | "warn"; text: string; fixed: boolean };
export type LintMeta = { original: string | null; content: string; findings: LintFinding[]; fixedRules: string[]; at: number };

/** 지금 글에 해당하는 linter 결과만 (그 뒤 플러그인·사용자가 글을 바꿨거나 다른 스와이프면 숨김) */
export function lintOf(m: Message): LintMeta | null {
	const l = (m.meta as { lint?: LintMeta } | null)?.lint;
	return l && typeof l === "object" && Array.isArray(l.findings) && l.content === m.content ? l : null;
}
