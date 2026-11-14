<script lang="ts">
	import { onMount } from "svelte";
	import { ConfirmHost, Toasts } from "$shared/ui";
	import CharacterPage from "$features/character/CharacterPage.svelte";
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
			{@const r = nav.current}
			{#if r.page === "character"}
				<CharacterPage charId={r.charId} convId={r.convId} />
			{:else}
				{#key r.page}
					<Placeholder page={r.page} />
				{/key}
			{/if}
		{/if}
	</main>
</div>
<Toasts />
<ConfirmHost />

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
