// gameSessionService.ts

import { gameEngineCommands, gameManagerCommands } from "$/ipc/commands";
import { runtimeBridge } from "../runtime/RuntimeBridge";
import { gameStore } from "../stores/gameStore.svelte";
import { journalStore } from "../stores/journalStore.svelte";
import { runtimeStore } from "../stores/runtimeStore.svelte";
import { sessionStore } from "../stores/sessionStore.svelte";


export async function startGameSession(gameId: string, sessionId: string) {
	const bundle = await gameManagerCommands.openGame({ gameId });
	const runtimeData = await gameEngineCommands.continueGame({ sessionId });
	const journal = await gameEngineCommands.readJournal({ sessionId, toSeq: 0 });

	gameStore.setBundle(bundle);
	sessionStore.setSession(sessionId);
	journalStore.setJournal(journal);

	await runtimeStore.initialize(runtimeData);
	await runtimeBridge.initialize(runtimeData);
}


export async function newGameSession(gameId: string) {
	const bundle = await gameManagerCommands.openGame({ gameId });
	const runtimeData = await gameEngineCommands.startNewGame();

	gameStore.setBundle(bundle);
	sessionStore.setSession(runtimeData.state.sessionId);

	await runtimeStore.initialize(runtimeData);
	await runtimeBridge.initialize(runtimeData);
	
	return runtimeData.state.sessionId;
}
