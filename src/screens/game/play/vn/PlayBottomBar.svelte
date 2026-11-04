<script lang="ts">
	import { MapPin, MoreHorizontal } from "@lucide/svelte";
	let { locationName, weatherName, bgmBlocked, onActions, onResumeBgm }: {
		locationName: string;
		weatherName: string;
		bgmBlocked: boolean;
		onActions: () => void;
		onResumeBgm: () => void;
	} = $props();
</script>

<footer class="bottom-bar">
	<span><MapPin size={15} /> {locationName}</span>
	<span>{weatherName}</span>
	<button class="more-button" onclick={onActions} aria-label="이동 및 행동 명령 보기"><MoreHorizontal size={18} /> 이동 / 행동</button>
	{#if bgmBlocked}<button class="bgm-unlock" onclick={onResumeBgm}>배경음 재생</button>{/if}
</footer>

<style>
	.bottom-bar { display: flex; align-items: center; gap: 20px; height: 44px; flex: 0 0 44px; padding: 0 22px; background: var(--game-footer-bg); border-top: 1px solid var(--game-border-subtle); color: var(--game-text-muted); font-size: 12px; }
	.bottom-bar span { display: inline-flex; align-items: center; gap: 6px; }
	.more-button { display: inline-flex; align-items: center; gap: 5px; margin-left: auto; padding: 6px 10px; border: 0; border-radius: var(--radius-sm); color: var(--game-text-secondary); background: var(--game-border-subtle); cursor: pointer; }
	.bgm-unlock { padding: 6px 9px; border: 1px solid var(--game-accent-border); border-radius: var(--radius-sm); color: var(--game-accent-strong); background: var(--game-accent-soft); cursor: pointer; }
	@media (max-width: 620px) { .bottom-bar { padding: 0 12px; gap: 12px; } }
</style>
