// 플레이 화면 ↔ 오른쪽 패널 다리
// 패널(이동·행동, 지도)이 명령을 보낼 때 플레이 화면의 dispatch를 거쳐야 대사·연출이 무대에 그대로 나온다.
import type { GameCommand } from "../types";

class PlayBridge {
	/** 플레이 화면이 열려 있을 때만 있음 */
	dispatch = $state<((command: GameCommand) => void) | null>(null);
	/** 명령 처리 중 (버튼 잠금) */
	loading = $state(false);
}

export const playBridge = new PlayBridge();
