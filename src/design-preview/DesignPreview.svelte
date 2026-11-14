<script lang="ts">
	import {
		Gamepad2,
		Minus,
		PanelRight,
		Settings,
		Smartphone,
		Square,
		UserRound,
		Users,
		X,
	} from "@lucide/svelte";
	import { IconButton } from "$shared/ui";
	import { ui } from "./mock.svelte";
	import { ACCENTS, accentValue } from "$shared/ui/accents";
	import PhonePage from "./PhonePage.svelte";
	import CharacterPage from "./CharacterPage.svelte";
	import SettingsPage from "./SettingsPage.svelte";

	type Page = "phone" | "character" | "game" | "persona" | "settings";
	let page: Page = $state("character");
	let dark = $state(false);
	let accent = $state(ACCENTS[0].id);

	$effect(() => {
		document.documentElement.dataset.theme = dark ? "dark" : "light";
		document.documentElement.style.setProperty("--accent", accentValue(accent, dark));
	});

	const nav = [
		{ id: "phone", icon: Smartphone, label: "폰", badge: 3 },
		{ id: "character", icon: Users, label: "캐릭터" },
		{ id: "game", icon: Gamepad2, label: "게임" },
		{ id: "persona", icon: UserRound, label: "페르소나" },
	] as const;
</script>

<div class="app">
	<!-- 프로그램 헤더 = 타이틀바 + 메뉴 (모바일도 같은 구조) -->
	<header class="titlebar">
		<span class="logo" aria-hidden="true">S</span>
		<nav aria-label="주 메뉴">
			{#each nav as n (n.id)}
				<button class="tab" class:active={page === n.id} aria-current={page === n.id ? "page" : undefined} aria-label={n.label} title={n.label} onclick={() => (page = n.id)}>
					<n.icon />
					{#if "badge" in n && n.badge}<span class="dot" aria-label="새 메시지 {n.badge}"></span>{/if}
				</button>
			{/each}
			<button class="tab" class:active={page === "settings"} aria-current={page === "settings" ? "page" : undefined} aria-label="설정" title="설정" onclick={() => (page = "settings")}>
				<Settings />
			</button>
		</nav>
		<div class="drag" aria-hidden="true"></div>
		<!-- 오른쪽 패널이 열려 있으면 패널 폭만큼의 구역: 패널 탭 + 여닫기 + 창 버튼 (옵시디언 방식) -->
		<div class="right-zone" class:panel={ui.panelAvailable && ui.panelOpen}>
			{#if ui.panelAvailable && ui.panelOpen}
				<div class="ptabs" role="tablist" aria-label="오른쪽 패널">
					{#each ui.panelTabs as t (t.id)}
						<button
							role="tab"
							class="ptab"
							class:on={ui.panelTab === t.id}
							aria-selected={ui.panelTab === t.id}
							aria-label={t.label}
							title={t.label}
							onclick={() => (ui.panelTab = t.id)}
						>
							<t.icon />
						</button>
					{/each}
				</div>
			{/if}
			{#if ui.panelAvailable}
				<IconButton label={ui.panelOpen ? "오른쪽 패널 닫기" : "오른쪽 패널 열기"} size="sm" active={ui.panelOpen} onclick={() => (ui.panelOpen = !ui.panelOpen)}><PanelRight /></IconButton>
			{/if}
			<div class="win" aria-hidden="true">
				<span><Minus /></span><span><Square /></span><span class="close"><X /></span>
			</div>
		</div>
	</header>

	<div class="content">
		{#if page === "phone"}
			<PhonePage />
		{:else if page === "character"}
			<CharacterPage />
		{:else if page === "settings"}
			<SettingsPage bind:accent bind:dark />
		{:else}
			<div class="todo">이 화면은 다음 단계에서 만듦</div>
		{/if}
	</div>
</div>

<style>
	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}
	.titlebar {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		height: var(--titlebar-height);
		padding-left: var(--space-3);
		background: var(--bg);
		border-bottom: 1px solid var(--border);
		flex: none;
	}
	.logo {
		display: grid;
		place-items: center;
		width: 20px;
		height: 20px;
		margin-right: var(--space-2);
		border-radius: 6px;
		background: var(--accent);
		color: var(--text-on-accent);
		font-size: 11px;
		font-weight: 800;
	}
	nav {
		display: flex;
		gap: 2px;
	}
	.tab {
		position: relative;
		display: grid;
		place-items: center;
		width: 30px;
		height: 26px;
		padding: 0;
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text-muted);
		font-size: var(--text-sm);
		font-weight: 500;
		cursor: pointer;
	}
	.tab :global(svg) {
		width: 16px;
		height: 16px;
	}
	.tab:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.tab.active {
		background: var(--surface);
		color: var(--text);
		box-shadow: var(--shadow-sm);
	}
	.tab.active :global(svg) {
		color: var(--accent);
	}
	.dot {
		position: absolute;
		top: 3px;
		right: 5px;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--accent);
		border: 1.5px solid var(--bg);
	}
	.drag {
		flex: 1;
		align-self: stretch;
	}
	.right-zone {
		display: flex;
		align-items: center;
		align-self: stretch;
		gap: var(--space-1);
	}
	/* 패널 바로 위: 패널과 같은 폭·같은 경계선 */
	.right-zone.panel {
		width: var(--panel-width);
		padding-left: var(--space-2);
		border-left: 1px solid var(--border);
		margin-bottom: -1px;
		border-bottom: 1px solid var(--bg);
	}
	.ptabs {
		display: flex;
		gap: 2px;
		margin-right: auto;
	}
	.ptab {
		display: grid;
		place-items: center;
		width: 30px;
		height: 26px;
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
	}
	.ptab :global(svg) {
		width: 16px;
		height: 16px;
	}
	.ptab:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.ptab.on {
		background: var(--surface-active);
		color: var(--text);
	}
	.win {
		display: flex;
		margin-left: var(--space-1);
		align-self: stretch;
	}
	.win span {
		display: grid;
		place-items: center;
		width: 44px;
		color: var(--text-muted);
	}
	.win span:hover {
		background: var(--surface-hover);
	}
	.win .close:hover {
		background: var(--danger);
		color: white;
	}
	.win :global(svg) {
		width: 14px;
		height: 14px;
	}
	.content {
		flex: 1;
		min-height: 0;
	}
	.todo {
		display: grid;
		place-items: center;
		height: 100%;
		color: var(--text-muted);
	}
	/* 좁은 화면: 탭 글자 숨기고 아이콘만 */
	@media (max-width: 720px) {
		.win {
			display: none;
		}
	}
</style>
