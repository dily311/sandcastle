<script lang="ts">
	import { onMount } from "svelte";
	import { themeSetting } from "$/lib/stores/themeStore.svelte";
	import { bootstrapFeatures, featureManager } from "$/lib/features/feautreManager.svelte";
	import { setLucideProps } from "@lucide/svelte";
	import { widgetCommands } from "$/ipc/commands";
	import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
	import ContextMenu from "$/lib/contextmenu/ui/ContextMenu.svelte";

	let isInitialized = $state(false);
	// icon 기본 설정
	setLucideProps({
		size: 16,
		class: "svg-icon",
	});

	onMount(async () => {
		// 테마 설정 불러오기
		await themeSetting.init();

		// 켜진 기능 불러오기
		await bootstrapFeatures();

		widgetCommands.listen_widgetShown(() => {
			featureManager.getFeatures.map((v) => {
				if (v.enabled) v.shown?.();
			});
		});
		widgetCommands.listen_widgetHidden(() => {
			featureManager.getFeatures.map((v) => {
				if (v.enabled) v.disable?.();
			});
		});

		isInitialized = true;
	});

	$effect(() => {
		// 위젯이 모두 꺼졌다면 자동으로 윈도우 종료
		if (isInitialized && featureManager.getFeatures.filter(f => f.enabled).length === 0) {
			getCurrentWebviewWindow().close();
		}
	});
</script>

{#if isInitialized}
	<div class="app-wrap">
		{#each featureManager.getFeatures as item}
			{#if item.enabled}
				<item.component />
			{/if}
		{/each}
	</div>
	<ContextMenu />
{/if}

<style>
	.app-wrap {
		background: transparent;
		position: relative;
		width: 100dvw;
		height: 100dvh;
	}
</style>
