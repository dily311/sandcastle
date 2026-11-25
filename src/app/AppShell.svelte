<script lang="ts">
	import { checkOnStart } from "./updateCheck";
	import { watchAutoSync } from "./autoSync.svelte";
	import { onMount } from "svelte";
	import { ConfirmHost, ImageViewerHost, installImageZoom, Spinner, Toasts } from "$shared/ui";
	import { library } from "$features/character/library.svelte";
	import CharacterPage from "$features/character/CharacterPage.svelte";
	import JobProgress from "./JobProgress.svelte";
	import { captureGlobalErrors, logs } from "$shared/stores/log.svelte";
	import { lastRoute, nav, saveLastRoute } from "./nav.svelte";
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
	import PluginHost from "$features/community/PluginHost.svelte";
	import ImagesPage from "$features/images/ImagesPage.svelte";
	import HomePage from "$features/home/HomePage.svelte";
	import { watchTaskErrors } from "$shared/stores/taskLog";
	import { appSetting } from "$shared/stores/appSetting.svelte";

	let ready = $state(false);
	let bootStatus = $state("앱을 준비하는 중…");

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
		swipe = layout.narrow && !layout.drawer && t.clientX < 24 ? { x: t.clientX, y: t.clientY } : null;
	}
	function touchMove(e: TouchEvent) {
		if (!swipe) return;
		const t = e.touches[0];
		if (t.clientX - swipe.x > 60 && Math.abs(t.clientY - swipe.y) < 40) {
			layout.drawer = true;
			swipe = null;
		}
	}

	// 모든 그림은 눌러서 크게 보기 (링크·버튼 안의 그림은 제외, 이유는 imageZoom.ts)
	onMount(() => installImageZoom());

	$effect(() => {
		const r = nav.current;
		if (ready) saveLastRoute(r);
	});

	onMount(async () => {
		captureGlobalErrors();
		bootStatus = "테마를 불러오는 중…";
		await theme.init().catch((e) => logs.error("app", String(e)));
		logs.info("app", "시작");
		bootStatus = "설정과 플러그인을 불러오는 중…";
		await Promise.all([
			appSetting.load().catch((e) => logs.error("app", String(e))),
			plugins.load().catch((e) => logs.error("app", String(e))),
		]);
		bootStatus = "캐릭터와 대화를 불러오는 중…";
		await library.load().catch((e) => logs.error("app", String(e)));
		// 새 버전 확인: 설정을 켰을 때만 (기본 꺼짐, 1-E-2)
		checkOnStart(appSetting.value?.checkUpdatesOnStart ?? false);
		// 첫 화면: 홈부터 보기를 켰으면 홈, 아니면 마지막으로 보던 화면 (5-E-5)
		if (nav.current.page === "character" && !nav.canBack) {
			const last = appSetting.value?.home?.startOnHome ? { page: "home" as const } : lastRoute();
			if (last) void nav.replace(last);
		}
		ready = true;
		void recoverInterruptedChats();
		watchTaskErrors();
		// 자동 동기화: 동기화 플러그인·자동 동기화·폴더가 모두 있을 때만 타이머 (기본 꺼짐)
		watchAutoSync();
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

<div class="app sc-app" data-page={nav.current.page} class:desktop={!layout.mobile} class:mobile={layout.mobile}>
	<Titlebar />
	{#if !ready}
		<div class="boot-screen" role="status" aria-live="polite" out:fade={{ duration: 180 }}>
			<div class="boot-card">
				<span class="boot-brand">sandcastle</span>
				<Spinner size={28} />
				<p class="boot-msg">{bootStatus}</p>
			</div>
		</div>
	{:else}
		{@const r = nav.current}
		<div class="body" in:fade={{ duration: 160 }}>
			{#if layout.narrow}
				{#if layout.drawer}
					<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (Esc·뒤로로도 닫힘) -->
					<div class="scrim" transition:fade={{ duration: 150 }} onclick={() => layout.closeDrawer()}></div>
					<div class="drawer" transition:fly={{ x: -300, duration: 200 }}><Sidebar /></div>
				{/if}
			{:else if layout.sidebar}
				<Sidebar />
			{/if}
			<main class="content sc-main">
				{#if r.page === "home"}
					<HomePage />
				{:else if r.page === "character"}
					<CharacterPage charId={r.charId} convId={r.convId} edit={r.edit} />
				{:else if r.page === "game"}
					{#if plugins.on("game")}<GamePage gameId={r.gameId} sessionId={r.sessionId} edit={r.edit} />{:else}<PluginOff id="game" name="게임 모드" />{/if}
				{:else if r.page === "phone"}
					{#if plugins.on("phone")}<PhonePage app={r.app} convId={r.convId} />{:else}<PluginOff id="phone" name="폰" />{/if}
				{:else if r.page === "images"}
					<ImagesPage />
				{:else if r.page === "lorebook"}
					<LorebookPage bookId={r.bookId} />
				{:else if r.page === "persona"}
					<PersonaPage personaId={r.personaId} />
				{:else if r.page === "settings"}
					<SettingsPage tab={r.tab} edit={r.edit} />
				{/if}
			</main>
		</div>
	{/if}
</div>
<DropLayer />
<ExportPicker />
<Toasts />
<JobProgress />
<ConfirmHost />
<ImageViewerHost />
<PluginHost enabled={plugins.on("community")} />

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
		padding: var(--safe-top) env(safe-area-inset-right) var(--safe-bottom) env(safe-area-inset-left);
	}
	.scrim {
		position: fixed;
		inset: 0;
		z-index: var(--z-app-drawer);
		background: rgb(0 0 0 / 0.4);
	}
	.drawer {
		position: fixed;
		top: 0;
		bottom: 0;
		left: 0;
		z-index: calc(var(--z-app-drawer) + 1);
		display: flex;
		width: min(84vw, 300px);
		padding-top: var(--safe-top);
		padding-bottom: var(--safe-bottom);
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
	.boot-screen {
		flex: 1;
		min-height: 0;
		display: grid;
		place-items: center;
		background: var(--bg);
		color: var(--text);
	}
	.boot-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-6);
	}
	.boot-brand {
		font-size: 22px;
		font-weight: 700;
		letter-spacing: -0.02em;
		color: var(--text);
	}
	.boot-msg {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
</style>
