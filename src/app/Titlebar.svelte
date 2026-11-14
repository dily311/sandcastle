<script lang="ts">
	import { ChevronLeft, ChevronRight, Minus, PanelLeft, PanelRight, Square, X } from "@lucide/svelte";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { IconButton } from "$shared/ui";
	import { library } from "$features/character/library.svelte";
	import { layout } from "./layout.svelte";
	import { nav } from "./nav.svelte";
	import { panel } from "./panel.svelte";
	import { theme } from "./theme.svelte";

	let { showWindowControls = true }: { showWindowControls?: boolean } = $props();

	const appWindow = getCurrentWindow();
	const panelShown = $derived(panel.available && panel.open);

	const SETTINGS: Record<string, string> = {
		general: "기본 설정",
		theme: "테마",
		shortcut: "단축키",
		widget: "위젯",
		api: "API 키",
		preset: "프리셋",
		regex: "정규식",
		data: "데이터",
		log: "로그",
	};
	// 지금 화면 이름 (Claude Code 헤더처럼 가운데 왼쪽에 보임)
	const title = $derived.by(() => {
		const r = nav.current;
		switch (r.page) {
			case "character": {
				const ch = library.character(r.charId);
				if (!ch) return ["캐릭터"];
				if (r.edit) return [ch.name, "편집"];
				const conv = library.conversation(r.convId);
				return conv ? [ch.name, conv.title] : [ch.name];
			}
			case "persona":
				return ["페르소나", library.persona(r.personaId)?.name].filter(Boolean) as string[];
			case "settings":
				return ["설정", SETTINGS[r.tab ?? "general"]];
			case "phone":
				return ["폰"];
			case "game":
				return ["게임"];
		}
	});
</script>

<!-- 프로그램 헤더: 사이드바 · 뒤로/앞으로 · 화면 제목 · (빈 곳은 창 끌기) · 오른쪽 패널 · 창 버튼 -->
<header class="titlebar" class:frameless={theme.setting.isFrameless} data-tauri-drag-region>
	<IconButton label={layout.sidebar ? "사이드바 닫기 (Ctrl+B)" : "사이드바 열기 (Ctrl+B)"} size="sm" active={layout.sidebar} onclick={() => layout.toggleSidebar()}>
		<PanelLeft />
	</IconButton>
	<div class="history">
		<IconButton label="뒤로 (Alt+←)" size="sm" disabled={!nav.canBack} onclick={() => nav.back()}><ChevronLeft /></IconButton>
		<IconButton label="앞으로 (Alt+→)" size="sm" disabled={!nav.canForward} onclick={() => nav.forward()}><ChevronRight /></IconButton>
	</div>
	<h1 class="title" data-tauri-drag-region>
		{#each title as t, i (i)}
			{#if i > 0}<span class="sep" aria-hidden="true">/</span>{/if}
			<span class:last={i === title.length - 1}>{t}</span>
		{/each}
	</h1>
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
		padding-left: var(--space-2);
		background: var(--bg);
		border-bottom: 1px solid var(--border);
		flex: none;
		user-select: none;
	}
	.history {
		display: flex;
		margin: 0 var(--space-1);
	}
	.title {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		margin: 0 var(--space-2);
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--text-muted);
		white-space: nowrap;
	}
	.title span {
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.title .last {
		color: var(--text);
		font-weight: 600;
	}
	.sep {
		color: var(--text-faint);
	}
	.history :global(.icon-btn) {
		width: 24px;
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
