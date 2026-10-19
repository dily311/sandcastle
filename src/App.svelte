<script lang="ts">
	import { onMount } from "svelte";
	import Router from "svelte-spa-router";
	import { routes } from "$/app/routes";
	import { isDesktop } from "$ipc/windows";
	import Titlebar from "$components/Titlebar.svelte";
	import Tooltip from "$components/ui/Tooltip.svelte";
	import Toast from "$/components/ui/Toast.svelte";
	import Sidebar from "$components/Sidebar.svelte";
	import LoadingScreen from "./components/LoadingScreen.svelte";
	import { conversations } from "$/lib/chat/chatStore.svelte";
	import { personas } from "$/lib/chat/personaStore.svelte";
	import { characters } from "$/lib/chat/characterStore.svelte";
	import { appSetting } from "$/lib/stores/settingStore.svelte";
	import { themeSetting } from "$/lib/stores/themeStore.svelte";
	import { SEED_CHARACTERS } from "./data/seed";
	import { apiProfiles } from "./lib/stores/apiProfileStore.svelte";
	import { setLucideProps } from "@lucide/svelte";
	import { bootstrapFeatures } from "$lib/features/feautreManager.svelte";

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
{#if !isInitialized}
	<LoadingScreen />
{:else}
	<div class="app-wrap">
		<Sidebar />

		<!-- 메인 콘텐츠 영역 -->
		<main class="app-layout">
			<Router {routes} />
		</main>

		{#if isDesktop}
			<Tooltip />
		{/if}
		<Toast />
	</div>
{/if}

<style>
	.app-wrap {
		display: flex;
		flex-direction: row;
		position: relative;
		width: 100dvw;
		height: calc(100dvh - env(safe-area-inset-top, 0px));
		/* 상단 상태 바 공간 확보 */
		padding-top: env(safe-area-inset-top, 0px);
		background-color: var(--background-primary);
		/* background-color: color-mix(in oklab, var(--background-primary) 80%, transparent 10%); */
	}

	.app-layout {
		width: calc(100% - var(--ribbon-width));
		height: calc(100% - var(--titlebar-height));
		flex: 1;
		overflow-y: auto;
	}

	:global(body.is-frameless .app-layout) {
		height: 100%;
	}

	:global(body:not(.is-frameless) .app-header) {
		display: flex;
		align-items: center;
		height: var(--titlebar-height);
		padding: 4px 90px 4px 15px;
		vertical-align: middle;
		font-size: 14px;
		background-color: var(--background-secondary);
		border-bottom: 1px solid var(--border-color);
	}
</style>
