<script lang="ts">
	import Button from "$/components/ui/button/Button.svelte";
	import ButtonGroup from "$/components/ui/button/ButtonGroup.svelte";
	import { gameManagerCommands } from "$/ipc/commands";
	import type { GameProject } from "$type/game";
	import { Notebook } from "@lucide/svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import { onMount } from "svelte";
	let games = $state<GameProject[] | undefined>(undefined);

	// onMount(async () => {
	// 	games = await gameCommands.listGames();
	// });

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
		await gameManagerCommands.importGame({ zipPath: path });
	}
	async function handleExport(gameId: string) {
		await gameManagerCommands.exportGame({ gameId, outputDir: "" });
	}
</script>

<div class="game-page">
	<!-- <div>
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
	</div> -->
	<ButtonGroup>
		<Button variant="outline" size="icon">
			<Notebook />
		</Button>
		<Button variant="outline">
			버튼 테스트
		</Button>
		<Button variant="outline">
			버튼 테스트
		</Button>
	</ButtonGroup>
</div>

<style>
	.game-page {
		display: flex;
		height: 100%;
		overflow: hidden;
		justify-content: space-evenly;
	}
</style>
