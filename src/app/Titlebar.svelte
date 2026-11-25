<script lang="ts">
	import { ChevronDown, ChevronLeft, ChevronRight, Copy, CornerUpLeft, Download, Folder, FolderMinus, GitBranch, Menu, Minus, PanelLeft, PanelRight, Pencil, Pin, PinOff, Square, Trash2, UserRound, X } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { ContextMenu, IconButton, type MenuItem } from "$shared/ui";
	import PluginCommands from "$features/community/PluginCommands.svelte";
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
	import { jobs } from "$shared/stores/jobs.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";

	let { showWindowControls = true }: { showWindowControls?: boolean } = $props();

	const appWindow = getCurrentWindow();
	const panelShown = $derived(panel.available && panel.open);

	const SETTINGS = $derived<Record<string, string>>({
		general: $_("settings.tabs.general"),
		chat: $_("settings.tabs.chat"),
		phone: $_("settings.tabs.phone"),
		theme: $_("settings.tabs.theme"),
		shortcut: $_("settings.tabs.shortcut"),
		widget: $_("settings.tabs.widget"),
		api: $_("settings.tabs.api"),
		preset: $_("settings.tabs.preset"),
		regex: $_("settings.tabs.regex"),
		lorebook: $_("settings.tabs.lorebook"),
		data: $_("settings.tabs.data"),
		log: $_("settings.tabs.log"),
		plugins: $_("settings.tabs.plugins"),
		translate: $_("settings.tabs.translate"),
		tts: $_("settings.tabs.tts"),
		imagegen: $_("settings.tabs.imagegen"),
		activity: $_("settings.tabs.activity"),
		budget: $_("settings.tabs.budget"),
		game: $_("settings.tabs.game"),
		cg: $_("settings.tabs.cg"),
		sync: $_("settings.tabs.sync"),
		community: $_("settings.tabs.community"),
		lint: $_("settings.tabs.lint"),
		memory: $_("settings.tabs.memory"),
		summary: $_("settings.tabs.summary"),
		rewrite: $_("settings.tabs.rewrite"),
		ghostwrite: $_("settings.tabs.ghostwrite"),
		quickreply: $_("settings.tabs.quickreply"),
	});
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
			...(c.characterId ? [{ label: "캐릭터 페이지로", icon: UserRound, onselect: () => nav.go({ page: "character", charId: c.characterId ?? undefined }) }] : []),
			{ label: "이름 바꾸기", icon: Pencil, separator: !!c.characterId, onselect: startRename },
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

	/** 좁은 화면은 마지막 이름만 보이므로, 그 옆 ▾ 메뉴에서 상위 경로 전부로 바로 이동 (캐릭터 › 이름 › 대화) */
	const crumbMenu = $derived.by((): MenuItem[] => (layout.narrow ? title.slice(0, -1).flatMap((t) => (t.to ? [{ label: t.label, icon: ChevronLeft, onselect: () => nav.go(t.to!) }] : [])) : []));
	const fullMenu = $derived.by((): MenuItem[] => {
		const [first, ...rest] = menuItems;
		// 이동 항목과 대화 동작 사이에 구분선
		return crumbMenu.length && first ? [...crumbMenu, { ...first, separator: true }, ...rest] : [...crumbMenu, ...menuItems];
	});

	// 지금 화면 이름 (Claude Code 헤더처럼 가운데 왼쪽에 보임)
	// 누르면 그 자리로 가는 경로 (마지막은 지금 화면)
	type Crumb = { label: string; to?: Route };
	const title = $derived.by((): Crumb[] => {
		const r = nav.current;
		switch (r.page) {
			case "character": {
				const ch = library.character(r.charId);
				if (!ch) return [{ label: $_("nav.character") }];
				const top: Crumb[] = [{ label: $_("nav.character"), to: { page: "character" } }, { label: ch.name, to: { page: "character", charId: ch.id } }];
				if (r.edit) return [...top, { label: $_("common.edit") }];
				const conv = library.conversation(r.convId);
				return conv ? [...top, { label: conv.title }] : top;
			}
			case "persona": {
				const p = r.personaId ? library.persona(r.personaId) : undefined;
				return p ? [{ label: $_("nav.persona"), to: { page: "persona" } }, { label: p.alias?.trim() || p.name }] : [{ label: $_("nav.persona") }];
			}
			case "settings": {
				if (layout.narrow && !r.tab) return [{ label: $_("nav.settings") }];
				const tabId = r.tab ?? "general";
				const tabLabel = SETTINGS[tabId] ?? (tabId.startsWith("community:") ? tabId.slice(10) : tabId);
				const tab: Crumb = { label: tabLabel, to: { page: "settings", tab: tabId } };
				if (r.tab === "theme" && r.edit === "fonts") return [{ label: $_("nav.settings"), to: { page: "settings" } }, tab, { label: "Fonts" }];
				return r.edit ? [{ label: $_("nav.settings"), to: { page: "settings" } }, tab, { label: $_("common.edit") }] : [{ label: $_("nav.settings"), to: { page: "settings" } }, tab];
			}
			case "home":
				return [{ label: $_("nav.home") }];
			case "phone": {
				const PHONE_APPS: Record<string, string> = {
					home: $_("nav.home"),
					messages: "메시지",
					contacts: "연락처",
					photos: "사진",
					camera: "카메라",
					settings: $_("nav.settings"),
				};
				if (r.convId) {
					const c = library.conversation(r.convId);
					const ch = c?.characterId ? library.character(c.characterId) : undefined;
					return [
						{ label: $_("nav.phone"), to: { page: "phone" } },
						{ label: "메시지", to: { page: "phone", app: "messages" } },
						{ label: ch?.name ?? c?.title ?? "채팅" },
					];
				}
				if (r.app && r.app !== "home") {
					return [{ label: $_("nav.phone"), to: { page: "phone" } }, { label: PHONE_APPS[r.app] ?? r.app }];
				}
				return [{ label: $_("nav.phone") }];
			}
			case "images":
				return [{ label: $_("nav.images") }];
			case "lorebook": {
				const b = lorebook.book(r.bookId);
				return b ? [{ label: $_("nav.lorebook"), to: { page: "lorebook" } }, { label: b.name }] : [{ label: $_("nav.lorebook") }];
			}
			case "game": {
				const g = games.game(r.gameId);
				if (!g) return [{ label: $_("nav.game") }];
				const top: Crumb[] = [{ label: $_("nav.game"), to: { page: "game" } }, { label: g.title, to: { page: "game", gameId: g.id } }];
				return r.sessionId ? [...top, { label: "Play" }] : r.edit ? [...top, { label: $_("common.edit") }] : top;
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

	/** 닫기: 파일 작업이 남았으면 알리고 (대기열은 메모리에만 — 끄면 기다리던 것은 하지 않음, 5-E-28) */
	async function closeApp() {
		const n = jobs.pending;
		if (n && !(await confirmAction({ title: "작업이 남아 있음", message: `파일 작업 ${n}개가 끝나지 않았습니다. 지금 끄면 하던 것은 멈추고 기다리던 것은 하지 않습니다.`, confirmLabel: "그래도 끄기", danger: true }))) return;
		await appWindow.close();
	}
</script>

<!-- 프로그램 헤더: 사이드바 · 뒤로/앞으로 · 화면 제목 · (빈 곳은 창 끌기) · 오른쪽 패널 · 창 버튼 -->
<header class="titlebar sc-titlebar" class:frameless={theme.setting.isFrameless} class:desktop={!layout.mobile} class:mobile={layout.mobile} class:narrow={layout.narrow} data-tauri-drag-region>
	{#if layout.narrow}
		<!-- 휴대폰: 메뉴 · 뒤로 · 지금 화면 이름 · 패널 -->
		<IconButton label="메뉴" onclick={() => layout.toggleSidebar()}><Menu /></IconButton>
		{#if nav.canBack}<IconButton label="뒤로" onclick={() => nav.back()}><ChevronLeft /></IconButton>{/if}
	{:else}
		<IconButton label={layout.sidebar ? "사이드바 닫기 (Ctrl+B)" : "사이드바 열기 (Ctrl+B)"} size="sm" active={layout.sidebar} onclick={() => layout.toggleSidebar()}>
			<PanelLeft />
		</IconButton>
		<div class="history">
			<IconButton label="뒤로 (Alt+←)" size="sm" disabled={!nav.canBack} onclick={() => nav.back()}><ChevronLeft /></IconButton>
			<IconButton label="앞으로 (Alt+→)" size="sm" disabled={!nav.canForward} onclick={() => nav.forward()}><ChevronRight /></IconButton>
		</div>
	{/if}
	<h1 class="title" data-tauri-drag-region>
		{#each layout.narrow ? title.slice(-1) : title as t, j (j)}
			{@const i = layout.narrow ? title.length - 1 : j}
			{#if j > 0}<span class="sep" aria-hidden="true">/</span>{/if}
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
				<IconButton label={layout.narrow ? "상위 화면·대화 동작" : "대화 동작"} size="sm" onclick={openMenu}><ChevronDown /></IconButton>
			{:else if t.to && i < title.length - 1}
				<button class="crumb" title="{t.label}(으)로" onclick={() => nav.go(t.to!)}>{t.label}</button>
			{:else}
				<span class:last={i === title.length - 1}>{t.label}</span>
			{/if}
		{/each}
		{#if layout.narrow && !conv && crumbMenu.length}
			<IconButton label="상위 화면" size="sm" onclick={openMenu}><ChevronDown /></IconButton>
		{/if}
	</h1>
	<ContextMenu at={menu} items={fullMenu} label="화면 이동" onclose={() => (menu = null)} />
	<div class="drag" data-tauri-drag-region></div>

	<PluginCommands />
	{#if panel.available}
		<IconButton label={panel.open ? "오른쪽 패널 닫기" : "오른쪽 패널 열기"} size={layout.narrow ? "md" : "sm"} active={panel.open} onclick={() => panel.toggle()}>
			<PanelRight />
		</IconButton>
	{/if}
	{#if !layout.narrow}
	<div class="right-zone" class:panel={panelShown}>
		{#if panelShown}
			<div class="ptabs" role="tablist" aria-label="오른쪽 패널" onwheel={wheelTabs}>
				{#each panel.allTabs as t (t.id)}
					<button
						role="tab"
						class="ptab"
						class:on={panel.tab === t.id}
						aria-selected={panel.tab === t.id}
						aria-label={t.label}
						title="{t.label} · 끌어서 순서 바꾸기"
						onclick={() => (panel.tab = t.id)}
						{...panel.tabDrag(t.id)}
					>
						<t.icon />
					</button>
				{/each}
			</div>
		{/if}
		{@render winButtons()}
	</div>
	{:else}
		<!-- 좁은 창(모바일 레이아웃)이어도 데스크톱 앱이면 창 버튼은 남긴다 -->
		{@render winButtons()}
	{/if}
</header>

{#snippet winButtons()}
	{#if showWindowControls && theme.setting.isFrameless && layout.desktop}
		<div class="win">
			<button aria-label="최소화" title="최소화" onclick={() => appWindow.minimize()}><Minus /></button>
			<button aria-label="최대화" title="최대화" onclick={() => appWindow.toggleMaximize()}><Square /></button>
			<button class="close" aria-label="닫기" title="닫기" onclick={closeApp}><X /></button>
		</div>
	{/if}
{/snippet}

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
	/* 좁은 화면 배치 (위 마크업의 layout.narrow 분기와 같은 기준). 높이만 휴대폰(.mobile)에서 크게 — tokens.css */
	.titlebar.narrow {
		padding: 0 var(--space-1);
	}
	.titlebar.narrow .title {
		flex: 1;
		font-size: var(--text-md, 15px);
		margin: 0 var(--space-1);
	}
	.titlebar.narrow .crumb {
		max-width: 100%;
	}
	.titlebar.narrow .drag {
		display: none;
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
		gap: 1px;
		flex: 1;
		min-width: 0;
		margin-right: auto;
		overflow-x: auto;
		scrollbar-width: none;
		mask-image: linear-gradient(to right, transparent 0, #000 6px, #000 calc(100% - 10px), transparent 100%);
		-webkit-mask-image: linear-gradient(to right, transparent 0, #000 6px, #000 calc(100% - 10px), transparent 100%);
		padding: 0 4px;
	}
	.ptabs::-webkit-scrollbar {
		display: none;
	}
	.ptab {
		flex: none;
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
		transition:
			background var(--duration-fast) var(--ease),
			color var(--duration-fast) var(--ease);
	}
	.ptab :global(svg) {
		width: 15px;
		height: 15px;
	}
	.ptab:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.ptab.on {
		background: var(--accent-soft);
		color: var(--accent-text);
	}
	.ptab.on :global(svg) {
		color: var(--accent);
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
