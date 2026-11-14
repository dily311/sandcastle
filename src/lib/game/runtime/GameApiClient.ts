import { isGameInit, type GameScriptMessage, type HostMessage } from "../host/protocol";

/**
 * iframe이 로드되자마자 호출합니다. 호스트에게 "준비됐다"고 알리고,
 * 호스트가 DB에서 읽어온 씬 데이터(GAME_INIT)를 보내줄 때까지 기다립니다.
 */
export function waitForInit(): Promise<HostMessage> {
	return new Promise((resolve) => {
		function handleMessage(event: MessageEvent) {
			if (!isGameInit(event.data)) return;
			window.removeEventListener("message", handleMessage);
			resolve(event.data);
		}

		window.addEventListener("message", handleMessage);

		const ready: GameScriptMessage = { type: "GAME_READY" };
		window.parent.postMessage(ready, "*");
	});
}
