import { gameManagerCommands } from "$/ipc/commands";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { toasts } from "../stores/toast.svelte";
import { push } from "svelte-spa-router";
import type { GameProject } from "../type/game";


// 새로만들기 버튼 클릭 시 (create_game 커맨드 연동 예시)
export async function handleCreateNewGame() {
	const title = prompt("새 게임 프로젝트의 제목을 입력하세요:");
	if (!title) return;

	const GameProject = await gameManagerCommands.createGame({ title, templateId: "dating-sim" });
	// 생성 후 바로 에디터로 이동
	push(`/game/${GameProject.id}/editor`);
}

export async function loadGames() {
	const games = await gameManagerCommands.getGameList();
	return games;
}

export async function handleExportGamePackage(gameId: string) {
	const exportPath = await gameManagerCommands.exportGame({ gameId });
	toasts.showToast({ type: "success", message: `out: ${exportPath}` });
	// 출력된 위치 표시
	await revealItemInDir(exportPath);
}

