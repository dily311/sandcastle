<script lang="ts">
	import { Spinner } from "$shared/ui";
	import GameLibrary from "./GameLibrary.svelte";
	import GameDetail from "./GameDetail.svelte";
	// 플레이 화면은 엔진의 공통 기능(대본·선택지·대화·지도·소지품)을 다 다루므로 모든 템플릿이 같이 씀
	import PlayView from "./play/vn/VisualNovelTemplatePlay.svelte";
	import GameEditor from "./editor/GameEditor.svelte";
	import { games } from "./game.svelte";

	// 게임 목록 → 게임 상세(플레이 기록) → 플레이 / 편집
	let { gameId, sessionId, edit = false }: { gameId?: string; sessionId?: string; edit?: boolean } = $props();

	$effect(() => {
		if (!games.loaded) void games.load();
	});

</script>

{#if !games.loaded}
	<div class="center"><Spinner /></div>
{:else if gameId && sessionId}
	{#key sessionId}<PlayView {gameId} {sessionId} />{/key}
{:else if gameId && edit}
	{#key gameId}<GameEditor {gameId} />{/key}
{:else if gameId}
	{#key gameId}<GameDetail {gameId} />{/key}
{:else}
	<GameLibrary />
{/if}

<style>
	.center {
		display: grid;
		place-items: center;
		height: 100%;
	}
</style>
