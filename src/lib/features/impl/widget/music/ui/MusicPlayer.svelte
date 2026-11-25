<script lang="ts">
	import MiniPlayer from "./MiniPlayer.svelte";
	import DefaultPlayer from "./DefaultPlayer.svelte";
	import { audioStore, widgetMusicSetting } from "../musicPlyerStore.svelte";
	import { onDestroy, onMount } from "svelte";

	onMount(async () => {
		await widgetMusicSetting.init();
	});

	onDestroy(() => {
		audioStore.stop();
	});
</script>

<div>
	{#if widgetMusicSetting.isMiniPlayer}
		<MiniPlayer />
	{:else}
		<DefaultPlayer />
	{/if}
</div>
