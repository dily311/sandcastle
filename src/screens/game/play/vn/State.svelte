<script lang="ts">
	import { gameStore } from "$/lib/game/gameStore.svelte";
</script>

<div class="state-box">
	<div class="metrics">
		<span>📍 맵: <code>{gameStore.currentMap?.name}</code></span>
		<span>🚩 위치: <code>{gameStore.currentLocation?.name || "없음"}</code></span>
		<span>⏰ 시간: <strong>{gameStore.gameTime}분</strong></span>
		<span>⛅ 날씨: <strong>{gameStore.currentWeather?.name}</strong></span>
		<span>💬 활성 NPC: <strong>{gameStore.activeNpc?.length || "없음"}</strong></span>
	</div>
</div>

<style>
	.state-box {
		display: flex;
		gap: 4px;
		position: fixed;

		right: 1rem;
	}

	.metrics {
		display: flex;
		gap: 1.2rem;
		font-size: 0.85rem;
		color: var(--text-muted);
	}
	.metrics strong {
		color: var(--text-bold);
	}
</style>
