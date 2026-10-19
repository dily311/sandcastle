<script lang="ts">
	import { gameCommands } from "$/ipc/commands";
	import type { GameProject } from "$/lib/types/game";
	import { open } from "@tauri-apps/plugin-dialog";
	import { onMount } from "svelte";

	// import GamePlayer from "$/lib/game/GamePlayer.svelte";
	// import type { NpcResponder } from "$/lib/game/npc/npcResponder";
	// let gameId: string = 'game_yuna_test';
	// let startSceneKey: string = 'yuna_intro';
	// let npcResponder: NpcResponder = {
	// 	send: async (args) => {
	// 		let random = Date.now();
	// 		// ai 메시지 응답 모킹
	// 		return {
	// 			speakerName: "char212",
	// 			text: `
	// 			반가워. 오늘 하루 어땠어? ${random}
	// 			<state>affection_yuna += 2</state>
	// 			계속 얘기해볼까 아니면 다른 얘기 할까?
	// 			<choice>더 얘기하고 싶어</choice>
	// 			<choice>나중에 다시 올게</choice>
	// 			`,
	// 		}
	// 	}
	// }

	let games = $state<GameProject[] | undefined>(undefined);

	onMount(async () => {
		games = await gameCommands.listGames();
	});

	async function handleImport() {
		const path = await open({
			directory: false,
			multiple: false,
			filters: [
				{
					name: "Images",
					extensions: ["zip", "t.game"],
				},
			],
		});
		if (!path) return;
		await gameCommands.importGamePack({ path });
	}
	async function handleExport(gameId: string) {
		await gameCommands.exportGamePack({ gameId });
	}
</script>

<div class="game-page">
	<div>
		<button type="button" onclick={handleImport}> 파일 임포트 </button>
	</div>
	<div>
		<table>
			<thead>
				<tr>
					<th>이미지</th>
					<th>id</th>
					<th>타이틀</th>
					<th>설명</th>
					<th>게임 api 버전</th>
					<th>게임 템플릿명</th>
					<th>생성일</th>
					<th>수정일</th>
					<th></th>
				</tr>
			</thead>
			<tbody>
				{#if games}
					{#each games as item}
						<tr>
							<td>{item.coverImage}</td>
							<td>{item.id}</td>
							<td>{item.title}</td>
							<td>{item.description}</td>
							<td>{item.apiVersion}</td>
							<td>{item.templateId}</td>
							<td>{item.settingJson}</td>
							<td>{item.createdAt}</td>
							<td>{item.updatedAt}</td>
							<td><button type="button" onclick={() => handleExport(item.id)}>내보내기</button></td>
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</div>
	<!-- <GamePlayer gameId={gameId} gameTitle={"유나와 대화"} startSceneKey={startSceneKey} npcResponder={npcResponder} /> -->
</div>

<style>
	.game-page {
		display: flex;
		height: 100%;
		overflow: hidden;
		justify-content: space-evenly;
	}
</style>
