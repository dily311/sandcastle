<script lang="ts">
	import {
		ChevronLeft,
		ChevronRight,
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
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { IconButton } from "$shared/ui";
	import { nav, type Page } from "./nav.svelte";
	import { panel } from "./panel.svelte";
	import { theme } from "./theme.svelte";

	let { showWindowControls = true, unread = 0 }: { showWindowControls?: boolean; unread?: number } = $props();

	const appWindow = getCurrentWindow();
	const items: { page: Page; label: string; icon: typeof Users }[] = [
		{ page: "phone", label: "폰", icon: Smartphone },
		{ page: "character", label: "캐릭터", icon: Users },
		{ page: "game", label: "게임", icon: Gamepad2 },
		{ page: "persona", label: "페르소나", icon: UserRound },
		{ page: "settings", label: "설정", icon: Settings },
	];
	const panelShown = $derived(panel.available && panel.open);
</script>

<!-- 프로그램 헤더: 로고 · 앞으로/뒤로 · 메뉴 · (빈 곳은 창 끌기) · 오른쪽 패널 탭 · 창 버튼 -->
<header class="titlebar" class:frameless={theme.setting.isFrameless} data-tauri-drag-region>
	<span class="logo" aria-hidden="true">S</span>
	<div class="history">
		<IconButton label="뒤로 (Alt+←)" size="sm" disabled={!nav.canBack} onclick={() => nav.back()}><ChevronLeft /></IconButton>
		<IconButton label="앞으로 (Alt+→)" size="sm" disabled={!nav.canForward} onclick={() => nav.forward()}><ChevronRight /></IconButton>
	</div>
	<nav aria-label="주 메뉴">
		{#each items as it (it.page)}
			<button
				class="tab"
				class:active={nav.current.page === it.page}
				aria-current={nav.current.page === it.page ? "page" : undefined}
				aria-label={it.label}
				title={it.label}
				onclick={() => nav.go({ page: it.page } as never)}
			>
				<it.icon />
				{#if it.page === "phone" && unread}<span class="dot" aria-label="새 메시지 {unread}"></span>{/if}
			</button>
		{/each}
	</nav>
	<div class="drag" data-tauri-drag-region></div>

	{#if panel.available}
		<IconButton label={panel.open ? "오른쪽 패널 닫기" : "오른쪽 패널 열기"} size="sm" active={panel.open} onclick={() => panel.toggle()}>
			<PanelRight />
		</IconButton>
	{/if}
	<div class="right-zone" class:panel={panelShown}>
		{#if panelShown}
			<div class="ptabs" role="tablist" aria-label="오른쪽 패널">
				{#each panel.tabs as t (t.id)}
					<button
						role="tab"
						class="ptab"
						class:on={panel.tab === t.id}
						aria-selected={panel.tab === t.id}
						aria-label={t.label}
						title={t.label}
						onclick={() => (panel.tab = t.id)}
					>
						<t.icon />
					</button>
				{/each}
			</div>
		{/if}
		{#if showWindowControls && theme.setting.isFrameless}
			<div class="win">
				<button aria-label="최소화" title="최소화" onclick={() => appWindow.minimize()}><Minus /></button>
				<button aria-label="최대화" title="최대화" onclick={() => appWindow.toggleMaximize()}><Square /></button>
				<button class="close" aria-label="닫기" title="닫기" onclick={() => appWindow.close()}><X /></button>
			</div>
		{/if}
	</div>
</header>

<style>
	.titlebar {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		height: var(--titlebar-height);
		padding-left: var(--space-3);
		background: var(--bg);
		border-bottom: 1px solid var(--border);
		flex: none;
		user-select: none;
	}
	.logo {
		display: grid;
		place-items: center;
		width: 20px;
		height: 20px;
		border-radius: 6px;
		background: var(--accent);
		color: var(--text-on-accent);
		font-size: 11px;
		font-weight: 800;
	}
	.history {
		display: flex;
		margin: 0 var(--space-2) 0 var(--space-1);
	}
	.history :global(.icon-btn) {
		width: 24px;
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
	.right-zone.panel {
		width: var(--panel-width);
		margin-left: var(--space-1);
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
		align-self: stretch;
		margin-left: var(--space-1);
	}
	.win button {
		display: grid;
		place-items: center;
		width: 44px;
		border: none;
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
	}
	.win button:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.win .close:hover {
		background: var(--danger);
		color: white;
	}
	.win :global(svg) {
		width: 14px;
		height: 14px;
	}
</style>
