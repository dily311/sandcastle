<script lang="ts">
	import { onMount } from "svelte";
	import { ConfirmHost, Toasts } from "$shared/ui";
	import CharacterPage from "$features/character/CharacterPage.svelte";
	import { captureGlobalErrors, logs } from "$shared/stores/log.svelte";
	import { nav } from "./nav.svelte";
	import { theme } from "./theme.svelte";
	import Titlebar from "./Titlebar.svelte";
	import Sidebar from "./Sidebar.svelte";
	import DropLayer from "./DropLayer.svelte";
	import ExportPicker from "./ExportPicker.svelte";
	import { layout } from "./layout.svelte";
	import SettingsPage from "$features/settings/SettingsPage.svelte";
	import PersonaPage from "$features/persona/PersonaPage.svelte";
	import PhonePage from "$features/phone/PhonePage.svelte";
	import GamePage from "$features/game/GamePage.svelte";
	import LorebookPage from "$features/lorebook/LorebookPage.svelte";
	import { fade, fly } from "svelte/transition";
	import { panel } from "./panel.svelte";
	import { recoverInterruptedChats } from "$features/character/recovery";
	import { plugins } from "$shared/stores/plugins.svelte";
	import PluginOff from "./PluginOff.svelte";
	import ImagesPage from "$features/images/ImagesPage.svelte";
	import HomePage from "$features/home/HomePage.svelte";
	import { watchTaskErrors } from "$shared/stores/taskLog";
	import { appSetting } from "$shared/stores/appSetting.svelte";

	let ready = $state(false);

	// 모바일: 다른 화면으로 가면 서랍과 패널 시트를 닫음
	$effect(() => {
		void nav.current;
		layout.closeDrawer();
		panel.closeSheet();
	});

	// 모바일: 화면 왼쪽 끝에서 오른쪽으로 쓸면 서랍 열기
	let swipe: { x: number; y: number } | null = null;
	function touchStart(e: TouchEvent) {
		const t = e.touches[0];
		swipe = layout.mobile && !layout.drawer && t.clientX < 24 ? { x: t.clientX, y: t.clientY } : null;
	}
	function touchMove(e: TouchEvent) {
		if (!swipe) return;
		const t = e.touches[0];
		if (t.clientX - swipe.x > 60 && Math.abs(t.clientY - swipe.y) < 40) {
			layout.drawer = true;
			swipe = null;
		}
	}

	onMount(async () => {
		captureGlobalErrors();
		await theme.init();
		logs.info("app", "시작");
		// 홈부터 보기를 켰으면 첫 화면을 홈으로 (기본 꺼짐)
		await appSetting.load();
		if (appSetting.value?.home?.startOnHome && nav.current.page === "character" && !nav.canBack) void nav.replace({ page: "home" });
		ready = true;
		void recoverInterruptedChats();
		void plugins.load();
		watchTaskErrors();
	});
</script>

<svelte:window
	ontouchstart={touchStart}
	ontouchmove={touchMove}
	onkeydown={(e) => {
		if (e.key === "Escape" && layout.drawer) return layout.closeDrawer();
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

<div class="app" class:mobile={layout.mobile}>
	<Titlebar />
	<div class="body">
		{#if layout.mobile}
			{#if layout.drawer}
				<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (Esc·뒤로로도 닫힘) -->
				<div class="scrim" transition:fade={{ duration: 150 }} onclick={() => layout.closeDrawer()}></div>
				<div class="drawer" transition:fly={{ x: -300, duration: 200 }}><Sidebar /></div>
			{/if}
		{:else if layout.sidebar}
			<Sidebar />
		{/if}
		<main class="content">
		{#if ready}
			{@const r = nav.current}
			{#if r.page === "home"}
				<HomePage />
			{:else if r.page === "character"}
				<CharacterPage charId={r.charId} convId={r.convId} edit={r.edit} />
			{:else if r.page === "game"}
				{#if plugins.on("game")}<GamePage gameId={r.gameId} sessionId={r.sessionId} edit={r.edit} />{:else}<PluginOff id="game" name="게임 모드" />{/if}
			{:else if r.page === "phone"}
				{#if plugins.on("phone")}<PhonePage />{:else}<PluginOff id="phone" name="폰" />{/if}
			{:else if r.page === "images"}
				<ImagesPage />
			{:else if r.page === "lorebook"}
				<LorebookPage bookId={r.bookId} />
			{:else if r.page === "persona"}
				<PersonaPage personaId={r.personaId} />
			{:else if r.page === "settings"}
				<SettingsPage tab={r.tab} edit={r.edit} />
			{/if}
		{/if}
		</main>
	</div>
</div>
<DropLayer />
<ExportPicker />
<Toasts />
<ConfirmHost />

<style>
	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
		height: 100dvh;
		overflow: hidden;
	}
	/* 휴대폰: 노치·홈 막대 피하기 */
	.app.mobile {
		padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left);
	}
	.scrim {
		position: fixed;
		inset: 0;
		z-index: 80;
		background: rgb(0 0 0 / 0.4);
	}
	.drawer {
		position: fixed;
		top: 0;
		bottom: 0;
		left: 0;
		z-index: 81;
		display: flex;
		width: min(84vw, 300px);
		padding-top: env(safe-area-inset-top);
		padding-bottom: env(safe-area-inset-bottom);
		background: var(--bg);
		box-shadow: var(--shadow-lg);
	}
	.drawer > :global(.sidebar) {
		width: 100%;
		border-right: none;
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
