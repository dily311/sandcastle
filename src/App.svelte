<script lang="ts">
	import { onMount } from "svelte";
	import Router from "svelte-spa-router";
	import { routes } from "$/app/routes";
	import { isDesktop } from "$ipc/windows";
	import Titlebar from "$components/Titlebar.svelte";
	import Tooltip from "$components/ui/Tooltip.svelte";
	import Toast from "$/components/ui/Toast.svelte";
	import Sidebar from "$components/Sidebar.svelte";
	import LoadingScreen from "./screens/LoadingScreen.svelte";
	import { conversations } from "$/lib/chat/chatStore.svelte";
	import { personas } from "$/lib/chat/personaStore.svelte";
	import { characters } from "$/lib/chat/characterStore.svelte";
	import { appSetting } from "$/lib/stores/settingStore.svelte";
	import { themeSetting } from "$/lib/stores/themeStore.svelte";
	import { SEED_CHARACTERS } from "./data/seed";
	import { apiProfiles } from "./lib/stores/apiProfileStore.svelte";
	import { setLucideProps } from "@lucide/svelte";
	import { bootstrapFeatures } from "$lib/features/feautreManager.svelte";
	import Header from "./components/Header.svelte";

	let isInitialized = $state(false);
	// icon 기본 설정
	setLucideProps({
		size: 16,
		class: "svg-icon",
	});
	onMount(async () => {
		await appSetting.init();
		await themeSetting.init();

		await apiProfiles.loadAll();
		await conversations.loadAll();
		await personas.loadAll();
		await characters.loadAll();

		if (characters && characters.value.length === 0) {
			SEED_CHARACTERS.map((char) => characters.create(char));
		}

		await bootstrapFeatures();

		isInitialized = true;
	});
</script>

{#if __IS_TAURI__ && isDesktop}
	<Titlebar />
{/if}
<div class="app-wrap">
	{#if !isInitialized}
		<LoadingScreen />
	{:else}
		<Sidebar />

		<!-- 메인 콘텐츠 영역 -->
		<main class="app-layout">
			<Router {routes} />
		</main>
	{/if}
</div>

{#if isDesktop}
	<Tooltip />
{/if}
<Toast />
