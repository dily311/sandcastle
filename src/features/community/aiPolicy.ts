import { get } from "svelte/store";
import { _ } from "svelte-i18n";

const tr = (key: string, values?: Record<string, string | number>) => get(_)(key, values ? { values } : undefined);

/** 사용자가 누른 명령 하나가 "사용자가 한 것"으로 부를 수 있는 AI 호출 수 */
export const COMMAND_AI_CALLS = 3;
/** 명령을 누른 뒤 그 몫이 유효한 시간 (ms) */
export const COMMAND_AI_WINDOW_MS = 60_000;
/** 확인 창을 거절한 플러그인이 다시 물을 수 있기까지 (ms) */
export const DENY_COOLDOWN_MS = 30_000;

export type Origin = {
	/** 호출한 프레임이 오른쪽 패널의 iframe인가 */
	fromPanel: boolean;
	/** 사용자가 방금 누른 명령의 몫이 남아 있나 (명령이 도는 중이고, 시간 안이고, 호출 수가 남음) */
	commandAllowance: boolean;
	/** 이 플러그인의 훅 이벤트(message.process·prompt.build)가 처리되는 중인가 */
	inHook: boolean;
	/** 플러그인별 "자동 AI 호출" 스위치 */
	aiAuto: boolean;
};

export type Decision = { by: "user" | "auto"; ask: boolean };

export function decide(o: Origin): Decision {
	if (o.inHook) return { by: "auto", ask: false };
	if (o.commandAllowance && !o.fromPanel) return { by: "user", ask: false };
	if (o.aiAuto) return { by: "auto", ask: false };
	if (o.fromPanel) return { by: "user", ask: true };
	return { by: "auto", ask: false };
}

const clip = (s: string, n: number) => (s.length > n ? `${s.slice(0, n)}…` : s);

/** 확인 창에 보일 글: 플러그인(이름과 id), 보낼 글과 시스템 지시의 앞부분, 길이, 출력 상한 */
export function describeCall(pluginName: string, pluginId: string, args: unknown) {
	const a = (args && typeof args === "object" ? args : {}) as { prompt?: unknown; system?: unknown; maxTokens?: unknown };
	const prompt = String(a.prompt ?? "").trim();
	const system = typeof a.system === "string" ? a.system.trim() : "";
	const max = typeof a.maxTokens === "number" && Number.isFinite(a.maxTokens) ? Math.min(2000, Math.max(1, Math.floor(a.maxTokens))) : 500;
	const total = prompt.length + system.length;
	return {
		title: tr("communityUi.aiPolicyTitle", { name: pluginName, id: pluginId }),
		message:
			`${tr("communityUi.aiPolicyIntro")}\n\n` +
			`${tr("communityUi.aiPolicyStats", { chars: total.toLocaleString(), max })}\n\n` +
			`${tr("communityUi.aiPolicyPromptLabel")}\n${clip(prompt, 300) || tr("communityUi.aiPolicyEmpty")}` +
			(system ? `\n\n${tr("communityUi.aiPolicySystemLabel")}\n${clip(system, 200)}` : ""),
	};
}
