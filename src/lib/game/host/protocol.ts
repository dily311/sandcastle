/** iframe에서 보내지는 메시지 */
export type GameScriptMessage =
	// iframe이 로드되자마자 보내는 "나 준비됐어" 신호.
	// 호스트는 이걸 받은 뒤에야 실제 씬 데이터를 보냄 (DB에서 불러오는 데
	// 시간이 걸릴 수 있으니, iframe이 먼저 기다리는 구조가 안전).
	| { type: "GAME_READY" }
	| { type: "GAME_API_CALL"; callId: string; method: string; args: unknown }
	| { type: "GAME_API_RESPONSE"; callId: string; result: unknown };

/** Host에서 iframe으로 보내는 메시지 */
export type HostMessage =
	// 호스트가 DB에서 조립한 씬 데이터를 iframe에 넘겨줄 때 사용
	| { type: "GAME_INIT"; scene: unknown; stateDefs: unknown; initialState?: unknown }
	| { type: "STATE_RESPONSE"; requestId: string; state: unknown }
	| { type: "ACTION_RESULT"; requestId: string; result: unknown }
	| { type: "GAME_CONTEXT_UPDATED"; context: unknown }
	| { type: "GAME_PAUSED" };

type Request = {
	resolve: (value: unknown) => void;
	reject: (error: Error) => void;
};
export const pendingRequests = new Map<string, Request>();

export function isGameReady(data: unknown): data is GameScriptMessage {
	return typeof data === "object" && data !== null && (data as { type?: unknown }).type === "GAME_READY";
}

export function isGameInit(data: unknown): data is HostMessage {
	return typeof data === "object" && data !== null && (data as { type?: unknown }).type === "GAME_INIT";
}

export function isGameApiCall(data: unknown): data is GameScriptMessage {
	return typeof data === "object" && data !== null && (data as { type?: unknown }).type === "GAME_API_CALL";
}

export function isGameApiResponse(data: unknown): data is GameScriptMessage {
	return typeof data === "object" && data !== null && (data as { type?: unknown }).type === "GAME_API_RESPONSE";
}
