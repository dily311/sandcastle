// 이 파일은 iframe 안(=신뢰하지 않는 게임 코드 쪽)에서 실행됩니다.
// 실제 제작자가 쓰게 될 최종 Game API는 이 파일의 패턴을 그대로 복사해서
// 네임스페이스(Game.sprite, Game.npc 등)만 늘려가면 됩니다.

import {
	isGameApiResponse,
	isGameInit,
	type GameApiCallMessage,
	type GameInitMessage,
	type GameReadyMessage,
} from "../host/protocol";
import type { GameState, StateChangeInstruction } from "../types";

const pending = new Map<string, (result: unknown) => void>();
let callCounter = 0;

function call<T>(method: string, args: unknown): Promise<T> {
	const callId = `call_${++callCounter}`;

	return new Promise<T>((resolve) => {
		pending.set(callId, resolve as (result: unknown) => void);

		const message: GameApiCallMessage = { type: "GAME_API_CALL", callId, method, args };
		// sandbox="allow-scripts"만 준 iframe은 origin이 항상 "null"이라
		// targetOrigin을 특정 값으로 지정할 수 없습니다. '*'를 쓰는 대신
		// 메시지 자체를 최소한으로 한정된 스키마로 유지하는 것으로 방어합니다.
		window.parent.postMessage(message, "*");
	});
}

window.addEventListener("message", (event: MessageEvent) => {
	if (!isGameApiResponse(event.data)) return;

	const resolve = pending.get(event.data.callId);
	if (!resolve) return;

	pending.delete(event.data.callId);
	resolve(event.data.result);
});

export const Game = {
	textbox: {
		show: (args: { nodeId: string; speaker?: string; text: string }) => call<void>("textbox.show", args),
	},
	choiceBox: {
		show: (args: { nodeId: string; choices: { text: string }[] }) => call<number>("choiceBox.show", args),
	},
	storage: {
		/** 마지막으로 "수동 저장"된 지점을 불러옵니다. 없으면 null. */
		load: () => call<{ nodeId: string; state: GameState } | null>("storage.load", null),
	},
	progress: {
		/**
		 * DB에 저장하는 게 아니라, 호스트가 "지금 플레이어 위치"를 알고 있게만
		 * 합니다. 플레이어가 저장 버튼을 눌렀을 때 이 값을 기준으로 실제 저장돼요.
		 */
		report: (args: { nodeId: string; state: GameState }) => call<void>("progress.report", args),
	},
	npc: {
		/**
		 * AI를 한 번 호출합니다. 한 줄이 아니라 여러 줄(lines)이 올 수 있고,
		 * 상태 변화(stateChanges)와 다음 선택지(choices)가 같이 파싱되어 옵니다.
		 * choices가 비어있으면 이 노드의 대화는 끝난 것으로 보면 됩니다.
		 */
		say: (args: { nodeId: string; characterId: string; userMessage: string; state: GameState }) =>
			call<{
				speakerName: string;
				lines: string[];
				stateChanges: StateChangeInstruction[];
				choices: string[];
			}>("npc.say", args),
	},
};

/**
 * iframe이 로드되자마자 호출합니다. 호스트에게 "준비됐다"고 알리고,
 * 호스트가 DB에서 읽어온 씬 데이터(GAME_INIT)를 보내줄 때까지 기다립니다.
 */
export function waitForInit(): Promise<GameInitMessage> {
	return new Promise((resolve) => {
		function handleMessage(event: MessageEvent) {
			if (!isGameInit(event.data)) return;
			window.removeEventListener("message", handleMessage);
			resolve(event.data);
		}

		window.addEventListener("message", handleMessage);

		const ready: GameReadyMessage = { type: "GAME_READY" };
		window.parent.postMessage(ready, "*");
	});
}
