import { push } from "svelte-spa-router";
import { gameStore } from "../stores/gameStore.svelte";
import { gameManagerCommands } from "$/ipc/commands";
import { newGameSession } from "./gameSessionService";

export const assetUrl = (path: string) => gameStore?.assetBaseUrl + path.split("/").map(encodeURIComponent).join("/");

export async function handleOpenGameDetailPage(gameId: string) {
	const bundle = await gameManagerCommands.openGame({gameId});
	gameStore.setBundle(bundle);

	push(`/game/${gameId}`);
}

export async function handleStartGamePlay(gameId: string) {
	const bundle = await gameManagerCommands.openGame({gameId});
	gameStore.setBundle(bundle);
	push(`/game/${gameId}/sessions`);
}

export async function handleOpenGameEditor(gameId: string) {
	push(`/game/${gameId}/editor`);
}
