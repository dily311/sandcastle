<script lang="ts">
	import { onMount } from "svelte";
	import { themeSetting } from "$/lib/stores/themeStore.svelte";
	import { bootstrapWidgetFeatures, widgetFeatures } from "$/lib/features/widgetFeatures.svelte";
	import { setLucideProps } from "@lucide/svelte";
	import { onEvent } from "$shared/ipc/events";
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
		await bootstrapWidgetFeatures();

		void onEvent("widget:shown", () => {
			widgetFeatures.getFeatures.map((v) => {
				if (v.enabled) v.shown?.();
			});
		});
		void onEvent("widget:hidden", () => {
			widgetFeatures.getFeatures.map((v) => {
				if (v.enabled) v.disable?.();
			});
		});

		isInitialized = true;
	});

	// 하위 기능이 모두 꺼지면 백엔드(update_feature_setting)가 이 창을 닫는다.
	// 창이 스스로 닫으면 백엔드는 열려 있는 줄 알고 닫힌 창에 메시지를 보내게 된다
</script>

{#if isInitialized}
	<div class="app-wrap">
		{#each widgetFeatures.getFeatures as item}
			{#if item.enabled}
				<item.component />
			{/if}
		{/each}
	</div>
	<ContextMenu />
{/if}

<style>
	.app-wrap {
		display: flex;
		flex-direction: row;
		background: transparent;
		position: relative;
		width: 100dvw;
		height: 100dvh;
	}
</style>
