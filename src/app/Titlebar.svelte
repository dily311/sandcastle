<script lang="ts">
	import { ChevronDown, ChevronLeft, ChevronRight, Copy, CornerUpLeft, Download, Folder, FolderMinus, GitBranch, Minus, PanelLeft, PanelRight, Pencil, Pin, PinOff, Square, Trash2, X } from "@lucide/svelte";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { ContextMenu, IconButton, type MenuItem } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";
	import { convActions } from "$features/character/actions";
	import { library } from "$features/character/library.svelte";
	import { games } from "$features/game/game.svelte";
	import { lorebook } from "$features/lorebook/lorebook.svelte";
	import { layout } from "./layout.svelte";
	import { nav, type Route } from "./nav.svelte";
	import { panel } from "./panel.svelte";
	import { theme } from "./theme.svelte";

	let { showWindowControls = true }: { showWindowControls?: boolean } = $props();

	const appWindow = getCurrentWindow();
	const panelShown = $derived(panel.available && panel.open);

	const SETTINGS: Record<string, string> = {
		general: "기본 설정",
		chat: "대화",
		phone: "폰",
		theme: "테마",
		shortcut: "단축키",
		widget: "위젯",
		api: "API 키",
		preset: "프리셋",
		regex: "정규식",
		data: "데이터",
		log: "로그",
	};
	// 대화 화면이면 그 대화 (제목 고치기·동작 메뉴)
	const conv = $derived(nav.current.page === "character" && !nav.current.edit ? library.conversation(nav.current.convId) : undefined);
	let renaming = $state(false);
	let draft = $state("");
	function startRename() {
		if (!conv) return;
		draft = conv.title;
		renaming = true;
	}
	async function commitRename() {
		if (!renaming || !conv) return;
		renaming = false;
		if (draft.trim() && draft.trim() !== conv.title) await library.rename(conv.id, draft);
	}
	function focusSelect(el: HTMLInputElement) {
		el.focus();
		el.select();
	}

	let menu = $state<{ x: number; y: number } | null>(null);
	const menuItems = $derived.by((): MenuItem[] => {
		const c = conv;
		if (!c) return [];
		const parent = (c.meta as { branch?: { parentId: string } } | null)?.branch?.parentId;
		const folders = library.foldersOf(c.characterId ?? "");
		return [
			{ label: "이름 바꾸기", icon: Pencil, onselect: startRename },
			{ label: c.pinned ? "고정 풀기" : "고정", icon: c.pinned ? PinOff : Pin, onselect: () => library.togglePin(c.id) },
			{
				label: "복제 (포크)",
				icon: Copy,
				separator: true,
				onselect: async () => {
					const d = await library.duplicate(c.id);
					if (d) nav.go({ page: "character", charId: d.characterId ?? undefined, convId: d.id });
				},
			},
			{
				label: "마지막 메시지에서 분기",
				icon: GitBranch,
				disabled: !c.messageCount,
				onselect: async () => {
					const [last] = await call(commands.getMessages(c.id, 1, null), { source: "chat" });
					if (!last) return;
					const b = await library.branch(c.id, last.id);
					nav.go({ page: "character", charId: b.characterId ?? undefined, convId: b.id });
					toasts.success("분기를 만듦");
				},
			},
			...(parent && library.conversation(parent)
				? [{ label: "갈라져 나온 대화로", icon: CornerUpLeft, onselect: () => nav.go({ page: "character", charId: c.characterId ?? undefined, convId: parent }) }]
				: []),
			...folders.map((f, i) => ({
				label: `폴더로 이동: ${f.name}`,
				icon: Folder,
				separator: i === 0,
				checked: c.folderId === f.id,
				onselect: () => library.move(c.id, f.id),
			})),
			...(c.folderId ? [{ label: "폴더에서 빼기", icon: FolderMinus, onselect: () => library.move(c.id, null) }] : []),
			{ label: "내보내기…", icon: Download, separator: true, onselect: () => convActions.exportMenu(c) },
			{ label: "삭제", icon: Trash2, danger: true, onselect: () => convActions.remove(c) },
		];
	});
	function openMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.left, y: r.bottom + 4 };
	}

	// 지금 화면 이름 (Claude Code 헤더처럼 가운데 왼쪽에 보임)
	// 누르면 그 자리로 가는 경로 (마지막은 지금 화면)
	type Crumb = { label: string; to?: Route };
	const title = $derived.by((): Crumb[] => {
		const r = nav.current;
		switch (r.page) {
			case "character": {
				const ch = library.character(r.charId);
				if (!ch) return [{ label: "캐릭터" }];
				const top: Crumb[] = [{ label: "캐릭터", to: { page: "character" } }, { label: ch.name, to: { page: "character", charId: ch.id } }];
				if (r.edit) return [...top, { label: "편집" }];
				const conv = library.conversation(r.convId);
				return conv ? [...top, { label: conv.title }] : top;
			}
			case "persona": {
				const p = library.persona(r.personaId);
				return p ? [{ label: "페르소나", to: { page: "persona" } }, { label: p.name }] : [{ label: "페르소나" }];
			}
			case "settings": {
				const tab: Crumb = { label: SETTINGS[r.tab ?? "general"], to: { page: "settings", tab: r.tab ?? "general" } };
				return r.edit ? [{ label: "설정", to: { page: "settings" } }, tab, { label: "편집" }] : [{ label: "설정", to: { page: "settings" } }, tab];
			}
			case "phone":
				return [{ label: "폰" }];
			case "lorebook": {
				const b = lorebook.book(r.bookId);
				return b ? [{ label: "로어북", to: { page: "lorebook" } }, { label: b.name }] : [{ label: "로어북" }];
			}
			case "game": {
				const g = games.game(r.gameId);
				if (!g) return [{ label: "게임" }];
				const top: Crumb[] = [{ label: "게임", to: { page: "game" } }, { label: g.title, to: { page: "game", gameId: g.id } }];
				return r.sessionId ? [...top, { label: "플레이" }] : r.edit ? [...top, { label: "편집" }] : top;
			}
		}
	});
	// 탭이 많아 넘치면 휠로 옆으로 넘김
	function wheelTabs(e: WheelEvent) {
		const el = e.currentTarget as HTMLElement;
		if (el.scrollWidth <= el.clientWidth || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
		e.preventDefault();
		el.scrollLeft += e.deltaY;
	}
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
			{#if conv && i === title.length - 1}
				{#if renaming}
					<input
						class="rename"
						bind:value={draft}
						use:focusSelect
						aria-label="대화 이름"
						onblur={commitRename}
						onkeydown={(e) => {
							if (e.key === "Enter") commitRename();
							if (e.key === "Escape") renaming = false;
						}}
					/>
				{:else}
					<button class="crumb last" title="눌러서 이름 바꾸기" onclick={startRename}>{t.label}</button>
				{/if}
				<IconButton label="대화 동작" size="sm" onclick={openMenu}><ChevronDown /></IconButton>
			{:else if t.to && i < title.length - 1}
				<button class="crumb" title="{t.label}(으)로" onclick={() => nav.go(t.to!)}>{t.label}</button>
			{:else}
				<span class:last={i === title.length - 1}>{t.label}</span>
			{/if}
		{/each}
	</h1>
	<ContextMenu at={menu} items={menuItems} label="대화 동작" onclose={() => (menu = null)} />
	<div class="drag" data-tauri-drag-region></div>

	{#if panel.available}
		<IconButton label={panel.open ? "오른쪽 패널 닫기" : "오른쪽 패널 열기"} size="sm" active={panel.open} onclick={() => panel.toggle()}>
			<PanelRight />
		</IconButton>
	{/if}
	<div class="right-zone" class:panel={panelShown}>
		{#if panelShown}
			<div class="ptabs" role="tablist" aria-label="오른쪽 패널" onwheel={wheelTabs}>
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
	.crumb {
		max-width: 360px;
		padding: 2px 6px;
		margin: 0 -6px;
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		font: inherit;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		cursor: text;
	}
	.crumb:not(.last) {
		color: inherit;
		cursor: pointer;
	}
	.crumb:hover {
		background: var(--surface-hover);
	}
	.rename {
		width: 260px;
		height: 24px;
		padding: 0 6px;
		border: 1px solid var(--accent);
		border-radius: var(--radius-xs);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-weight: 600;
		outline: none;
	}
	.title :global(.icon-btn) {
		width: 22px;
		height: 22px;
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
		min-width: 0;
		flex: none;
	}
	.right-zone.panel {
		width: var(--panel-width);
		margin-left: var(--space-1);
		padding-left: var(--space-2);
		border-left: 1px solid var(--border);
		margin-bottom: -1px;
		border-bottom: 1px solid var(--bg);
	}
	/* 탭이 많으면 줄어들고 옆으로 넘김 (창 버튼은 밀려나지 않게) */
	.ptabs {
		display: flex;
		gap: 2px;
		flex: 1;
		min-width: 0;
		margin-right: auto;
		overflow-x: auto;
		scrollbar-width: none;
	}
	.ptabs::-webkit-scrollbar {
		display: none;
	}
	.ptab {
		flex: none;
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
		flex: none;
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
