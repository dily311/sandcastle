// 보내기 전 프롬프트 훅 (CLAUDE.md 6-D-5, ADR-012). 백엔드가 보내기 직전 user-plugin-prompt로 묻는다.
// - 플러그인: sandcastle.on("prompt.build", (ctx) => ({ blocks: [{ text, position: "rules" | "history", depth? }] }))
// - 백엔드가 고른 플러그인(prompt.modify 권한)만 설치 순서로, 플러그인당 2초(백엔드 PER_PLUGIN_MS와 같게)
// - 블록을 더하기만 한다. 상한·검사는 백엔드가 한다
import type { PluginPromptAnswer, PromptHookRequest } from "$ipc/bindings";

type Block = { text: string; position: "rules" | "history"; depth: number | null };

export const PROMPT_EVENT = "prompt.build";
export const PER_PLUGIN_MS = 2000;

type Emit = (pluginId: string, event: string, data: unknown, timeoutMs: number) => Promise<unknown>;

function blocksOf(v: unknown): Block[] {
	const list = Array.isArray(v) ? v : v && typeof v === "object" ? (v as { blocks?: unknown }).blocks : null;
	if (!Array.isArray(list)) return [];
	return list
		.map((b): Block | null => {
			if (typeof b === "string") return { text: b, position: "rules", depth: null };
			if (!b || typeof b !== "object") return null;
			const o = b as { text?: unknown; position?: unknown; depth?: unknown };
			const depth = typeof o.depth === "number" && Number.isFinite(o.depth) && o.depth >= 0 ? Math.floor(o.depth) : null;
			return { text: String(o.text ?? ""), position: o.position === "history" ? "history" : "rules", depth };
		})
		.filter((b): b is Block => !!b && b.text.trim() !== "");
}

/** 플러그인들에게 차례로 묻는다 (순수 — 테스트한다). 실패·시간 초과는 빈 답 */
export async function gather(req: PromptHookRequest, running: readonly string[], emit: Emit) {
	const answers: PluginPromptAnswer[] = [];
	const errors: { pluginId: string; message: string }[] = [];
	const ctx = { conversationId: req.conversationId, characterId: req.characterId, userInput: req.userInput };
	for (const id of req.plugins) {
		if (!running.includes(id)) continue;
		try {
			const out = await emit(id, PROMPT_EVENT, ctx, PER_PLUGIN_MS);
			const blocks = blocksOf(out);
			if (blocks.length) answers.push({ pluginId: id, blocks });
		} catch (e) {
			errors.push({ pluginId: id, message: e instanceof Error ? e.message : String(e) });
		}
	}
	return { answers, errors };
}
