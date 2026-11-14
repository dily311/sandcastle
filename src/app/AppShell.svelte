<script lang="ts">
	import { onMount } from "svelte";
	import { ConfirmHost, Toasts } from "$shared/ui";
	import CharacterPage from "$features/character/CharacterPage.svelte";
	import { captureGlobalErrors, logs } from "$shared/stores/log.svelte";
	import { nav } from "./nav.svelte";
	import { theme } from "./theme.svelte";
	import Titlebar from "./Titlebar.svelte";
	import Sidebar from "./Sidebar.svelte";
	import { layout } from "./layout.svelte";
	import Placeholder from "./Placeholder.svelte";
	import SettingsPage from "$features/settings/SettingsPage.svelte";
	import PersonaPage from "$features/persona/PersonaPage.svelte";
	import PhonePage from "$features/phone/PhonePage.svelte";

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
		if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "b") {
			e.preventDefault();
			layout.toggleSidebar();
			return;
		}
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
	<div class="body">
		{#if layout.sidebar}<Sidebar />{/if}
		<main class="content">
		{#if ready}
			{@const r = nav.current}
			{#if r.page === "character"}
				<CharacterPage charId={r.charId} convId={r.convId} edit={r.edit} />
			{:else if r.page === "phone"}
				<PhonePage />
			{:else if r.page === "persona"}
				<PersonaPage personaId={r.personaId} />
			{:else if r.page === "settings"}
				<SettingsPage tab={r.tab} />
			{:else}
				{#key r.page}
					<Placeholder page={r.page} />
				{/key}
			{/if}
		{/if}
		</main>
	</div>
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
	.body {
		flex: 1;
		min-height: 0;
		display: flex;
	}
	.content {
		flex: 1;
		min-width: 0;
		min-height: 0;
	}
</style>
