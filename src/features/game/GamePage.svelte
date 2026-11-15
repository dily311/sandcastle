<script lang="ts">
	import { Spinner } from "$shared/ui";
	import GameLibrary from "./GameLibrary.svelte";
	import GameDetail from "./GameDetail.svelte";
	// 플레이 화면은 엔진의 공통 기능(대본·선택지·대화·지도·소지품)을 다 다루므로 모든 템플릿이 같이 씀
	import PlayView from "./play/vn/VisualNovelTemplatePlay.svelte";
	import GameEditor from "./editor/GameEditor.svelte";
	import { games } from "./game.svelte";
	import GamePanel from "./play/GamePanel.svelte";
	import SidePanel from "$app/SidePanel.svelte";
	import { panel } from "$app/panel.svelte";
	import { Activity, HeartHandshake, Package } from "@lucide/svelte";

	// 게임 목록 → 게임 상세(플레이 기록) → 플레이 / 편집
	let { gameId, sessionId, edit = false }: { gameId?: string; sessionId?: string; edit?: boolean } = $props();

	$effect(() => {
		if (!games.loaded) void games.load();
	});

	// 오른쪽 패널은 플레이 중에만
	const playing = $derived(!!(gameId && sessionId));
	$effect(() => {
		if (!playing) return;
		return panel.register([
			{ id: "status", label: "상태", icon: Activity },
			{ id: "relations", label: "관계", icon: HeartHandshake },
			{ id: "items", label: "소지품", icon: Package },
		]);
	});

</script>

{#if !games.loaded}
	<div class="center"><Spinner /></div>
{:else if gameId && sessionId}
	<div class="play">
		<div class="stage">{#key sessionId}<PlayView {gameId} {sessionId} />{/key}</div>
		{#if panel.open}<SidePanel label="게임 패널"><GamePanel /></SidePanel>{/if}
	</div>
{:else if gameId && edit}
	{#key gameId}<GameEditor {gameId} />{/key}
{:else if gameId}
	{#key gameId}<GameDetail {gameId} />{/key}
{:else}
	<GameLibrary />
{/if}

<style>
	.play {
		display: flex;
		height: 100%;
		min-height: 0;
	}
	/* 플레이 화면 안의 z-index(머리줄·알림 등)가 앱의 패널·서랍 위로 올라오지 않게 가둠 */
	.stage {
		position: relative;
		isolation: isolate;
		flex: 1;
		min-width: 0;
		min-height: 0;
	}
	.center {
		display: grid;
		place-items: center;
		height: 100%;
	}
</style>
