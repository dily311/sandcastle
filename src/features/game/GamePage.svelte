<script lang="ts">
	import { Spinner } from "$shared/ui";
	import GameLibrary from "./GameLibrary.svelte";
	import GameDetail from "./GameDetail.svelte";
	// 플레이·에디터는 아직 예전 화면을 씀 (색 변수는 legacy.css가 새 토큰에 연결)
	import "./legacy.css";
	import VisualNovelTemplatePlay from "$screens/game/play/vn/VisualNovelTemplatePlay.svelte";
	import DefaultTemplatePlay from "$screens/game/play/DefaultTemplatePlay.svelte";
	import GameEditorPage from "$screens/game/GameEditorPage.svelte";
	import { games } from "./game.svelte";

	// 게임 목록 → 게임 상세(플레이 기록) → 플레이 / 편집
	let { gameId, sessionId, edit = false }: { gameId?: string; sessionId?: string; edit?: boolean } = $props();

	$effect(() => {
		if (!games.loaded) void games.load();
	});

	// 템플릿별 플레이 화면
	const templateId = $derived(games.game(gameId)?.templateId ?? games.opened?.project.templateId ?? "");
	const Play = $derived(templateId === "dating-sim" ? VisualNovelTemplatePlay : DefaultTemplatePlay);
</script>

{#if !games.loaded}
	<div class="center"><Spinner /></div>
{:else if gameId && sessionId}
	<div class="legacy-game">
		{#key sessionId}<Play {gameId} {sessionId} />{/key}
	</div>
{:else if gameId && edit}
	<div class="legacy-game">
		{#key gameId}<GameEditorPage params={{ gameId }} />{/key}
	</div>
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
