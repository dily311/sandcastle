// 새 게임 시작 / 이어하기: 게임을 열고 런타임 상태를 채움
import { call, commands } from "$shared/ipc/client";
import type { ContinueGameResponse, OpenGameResponse } from "../types";
import { gameStore } from "./gameStore.svelte";
import { runtimeStore } from "./runtimeStore.svelte";

const src = { source: "game" };

async function open(gameId: string) {
	gameStore.setBundle((await call(commands.openGame(gameId), src)) as unknown as OpenGameResponse);
}

/** 새 세션을 만들고 그 id를 돌려줌 */
export async function newGameSession(gameId: string) {
	await open(gameId);
	// 오류(데이터 문제 등)는 부른 화면이 코드에 맞춰 알린다
	const data = (await call(commands.startNewGame(), { ...src, silent: true })) as unknown as Omit<ContinueGameResponse, "history">;
	await runtimeStore.initialize({ ...data, history: { entries: [], hasMore: false, oldestSeq: 0 } });
	return data.state.sessionId;
}

/** 저장된 세션 이어하기 */
export async function startGameSession(gameId: string, sessionId: string) {
	await open(gameId);
	await runtimeStore.initialize((await call(commands.continueGame(sessionId, null), src)) as unknown as ContinueGameResponse);
}
