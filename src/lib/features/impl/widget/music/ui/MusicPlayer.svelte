<script lang="ts">
	import MiniPlayer from "./MiniPlayer.svelte";
	import DefaultPlayer from "./DefaultPlayer.svelte";
	import { widgetMusicSetting } from "../musicPlyerStore.svelte";
	import { onMount } from "svelte";

	onMount(async () => {
		await widgetMusicSetting.init();
	});
</script>

<div>
	{#if widgetMusicSetting.isMiniPlayer}
		<MiniPlayer />
	{:else}
		<DefaultPlayer />
	{/if}
</div>
