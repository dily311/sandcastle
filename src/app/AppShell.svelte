<script lang="ts">
	import { onMount } from "svelte";
	import { Toasts } from "$shared/ui";
	import { captureGlobalErrors, logs } from "$shared/stores/log.svelte";
	import { nav } from "./nav.svelte";
	import { theme } from "./theme.svelte";
	import Titlebar from "./Titlebar.svelte";
	import Placeholder from "./Placeholder.svelte";

	let ready = $state(false);

	onMount(async () => {
		captureGlobalErrors();
		await theme.init();
		logs.info("app", "시작");
		ready = true;
	});
</script>

<svelte:window
	onkeydown={(e) => {
		if (!e.altKey) return;
		if (e.key === "ArrowLeft") nav.back();
		if (e.key === "ArrowRight") nav.forward();
	}}
	onmouseup={(e) => {
		if (e.button === 3) nav.back();
		if (e.button === 4) nav.forward();
	}}
/>

<div class="app">
	<Titlebar />
	<main class="content">
		{#if ready}
			{#key nav.current.page}
				<Placeholder page={nav.current.page} />
			{/key}
		{/if}
	</main>
</div>
<Toasts />

<style>
	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
		overflow: hidden;
	}
	.content {
		flex: 1;
		min-height: 0;
	}
</style>
