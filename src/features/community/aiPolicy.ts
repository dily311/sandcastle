// 플러그인 AI 호출의 실행 주체 (CLAUDE.md 6-D-7, ADR-012). 플러그인은 정하지 못하고 화면의 호스트가 본 것으로만 정한다.
//   - 훅(메시지 후처리·프롬프트) 안: 사용자가 누른 것이 아니므로 auto — 플러그인의 "자동 AI 호출" 스위치가 있어야 한다
//   - 사용자가 누른 명령이 도는 동안, 메인 프레임에서: user (사용자가 그 명령을 눌렀다)
//   - 그 밖(패널 등): 스위치가 켜져 있으면 auto, 아니면 호출마다 사용자에게 확인을 받고 허락하면 user
// 백엔드도 auto는 스위치를 다시 확인한다(꺼져 있으면 거절).
export type Origin = {
	/** 호출한 프레임이 오른쪽 패널의 iframe인가 */
	fromPanel: boolean;
	/** 이 플러그인의 명령 이벤트가 처리되는 중인가 */
	inCommand: boolean;
	/** 이 플러그인의 훅 이벤트(message.process·prompt.build)가 처리되는 중인가 */
	inHook: boolean;
	/** 플러그인별 "자동 AI 호출" 스위치 */
	aiAuto: boolean;
};

export type Decision = { by: "user" | "auto"; ask: boolean };

export function decide(o: Origin): Decision {
	if (o.inHook) return { by: "auto", ask: false };
	if (o.inCommand && !o.fromPanel) return { by: "user", ask: false };
	if (o.aiAuto) return { by: "auto", ask: false };
	return { by: "user", ask: true };
}

/** 확인 창에 보일 글 (프롬프트 앞부분과 상한) */
export function describeCall(pluginName: string, args: unknown) {
	const a = (args && typeof args === "object" ? args : {}) as { prompt?: unknown; system?: unknown; maxTokens?: unknown };
	const prompt = String(a.prompt ?? "").trim();
	const excerpt = prompt.length > 300 ? `${prompt.slice(0, 300)}…` : prompt;
	const max = typeof a.maxTokens === "number" && Number.isFinite(a.maxTokens) ? Math.min(2000, Math.max(1, Math.floor(a.maxTokens))) : 500;
	return {
		title: `'${pluginName}'이(가) AI를 부르려 합니다`,
		message: `연결된 AI API로 글을 만듭니다. 요금이 들 수 있습니다 (예산 한도가 있으면 그 안에서만).\n\n최대 출력 ${max} 토큰\n\n보낼 글:\n${excerpt || "(비어 있음)"}`,
	};
}
