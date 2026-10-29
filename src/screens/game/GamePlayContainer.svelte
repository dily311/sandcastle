<script lang="ts">
	import { onMount } from "svelte";

	// 템플릿별 UI 컴포넌트 임포트
	import DefaultPlay from "./play/DefaultTemplatePlay.svelte";
	import GachaGameLobby from "./play/gacha/GachaGameLobby.svelte";
	import VisualNovelTemplatePlay from "./play/VisualNovelTemplatePlay.svelte";

	type Props = {
		params: { sessionId: string; gameId: string };
	};
	let { params }: Props = $props();

	let templateId = $state<string>("dating-sim");
	let isLoading = $state(true);

	// 1. 세션 또는 게임 정보에서 templateId 가져오기
	async function loadGameTemplateInfo() {
		try {
			// 예: 세션 정보나 게임 매니페스트를 불러와서 templateId 획득
			// const manifest = await invoke('open_game', { gameId: params.gameId });
			// templateId = manifest.templateId;

			// 테스트용 임시 값
			templateId = "dating-sim";
		} catch (err) {
			console.error("Failed to load template info:", err);
		} finally {
			isLoading = false;
		}
	}

	// 2. templateId에 매핑되는 컴포넌트 선택
	let ActivePlayComponent = $derived.by(() => {
		switch (templateId) {
			//   case 'fantasy_rpg': return FantasyPlay;
			//   case 'cyberpunk': return CyberpunkPlay;
			case "dating-sim": return VisualNovelTemplatePlay;
			default:
				return DefaultPlay;
		}
	});

	onMount(() => {
		loadGameTemplateInfo();
	});
</script>

{#if isLoading}
	<div class="loading">플레이 환경을 불러오는 중...</div>
{:else}
	<!-- 동적 컴포넌트 렌더링 (Svelte 5) -->
	<ActivePlayComponent sessionId={params.sessionId} />
{/if}

<style>
	.loading {
		display: flex;
		justify-content: center;
		align-items: center;
		height: 100vh;
		color: #94a3b8;
	}
</style>
