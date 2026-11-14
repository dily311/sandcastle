<script lang="ts">
	import {
		BookOpenText,
		Gamepad2,
		Minus,
		Settings,
		Smartphone,
		Square,
		UserRound,
		Users,
		X,
	} from "@lucide/svelte";
	import { IconButton } from "$shared/ui";
	import { ACCENTS, accentValue } from "$shared/ui/accents";
	import PhonePage from "./PhonePage.svelte";
	import ChatPage from "./ChatPage.svelte";
	import SettingsPage from "./SettingsPage.svelte";

	type Page = "phone" | "chat" | "game" | "character" | "persona" | "settings";
	let page: Page = $state("chat");
	let dark = $state(false);
	let accent = $state(ACCENTS[0].id);

	$effect(() => {
		document.documentElement.dataset.theme = dark ? "dark" : "light";
		document.documentElement.style.setProperty("--accent", accentValue(accent, dark));
	});

	const nav = [
		{ id: "phone", icon: Smartphone, label: "폰", badge: 3 },
		{ id: "chat", icon: BookOpenText, label: "대화" },
		{ id: "game", icon: Gamepad2, label: "게임" },
		{ id: "character", icon: Users, label: "캐릭터" },
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
		</nav>
		<div class="drag" aria-hidden="true"></div>
		<IconButton label="설정" size="sm" active={page === "settings"} onclick={() => (page = "settings")}><Settings /></IconButton>
		<div class="win" aria-hidden="true">
			<span><Minus /></span><span><Square /></span><span class="close"><X /></span>
		</div>
	</header>

	<div class="content">
		{#if page === "phone"}
			<PhonePage />
		{:else if page === "chat"}
			<ChatPage />
		{:else if page === "settings"}
			<SettingsPage bind:accent bind:dark />
		{:else}
			<div class="todo">이 화면은 다음 단계에서 만듦 (캐릭터·게임은 소개 페이지부터)</div>
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
	.win {
		display: flex;
		margin-left: var(--space-2);
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
