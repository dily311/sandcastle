// 플러그인 AI 호출의 실행 주체 (CLAUDE.md 6-D-7, ADR-012). 플러그인은 정하지 못하고 화면의 호스트가 본 것으로만 정한다.
//   - 훅(메시지 후처리·프롬프트) 안: 사용자가 누른 것이 아니므로 auto — 플러그인의 "자동 AI 호출" 스위치가 있어야 한다
//   - 사용자가 방금 누른 명령의 몫이 남아 있을 때, 메인 프레임에서: user (누른 명령 하나에 몇 번까지만, 아래 COMMAND_AI_CALLS)
//   - 스위치가 켜져 있으면 auto
//   - 패널(눈에 보이는 화면)에서 부른 것만: 호출마다 사용자에게 확인을 받고 허락하면 user
//   - 그 밖(눈에 안 보이는 프레임의 타이머 등): auto — 스위치가 없으면 백엔드가 거절. 화면에 확인 창을 띄우지 않는다
// 백엔드도 auto는 스위치를 다시 확인한다(꺼져 있으면 거절).

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
		title: `'${pluginName}' (${pluginId})이(가) AI를 부르려 합니다`,
		message:
			`연결된 AI API로 글을 만듭니다. 요금이 들 수 있습니다 (예산 한도가 있으면 그 안에서만).\n\n` +
			`보낼 글 ${total.toLocaleString()}자 · 최대 출력 ${max} 토큰\n\n` +
			`보낼 글:\n${clip(prompt, 300) || "(비어 있음)"}` +
			(system ? `\n\n시스템 지시:\n${clip(system, 200)}` : ""),
	};
}
