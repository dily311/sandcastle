<script lang="ts">
	import { layout } from "$app/layout.svelte";
	import { _ } from "svelte-i18n";
	import type { Component } from "svelte";
	import { Activity, ArrowLeftRight, BookOpen, Gamepad2, Images, AudioLines, Blocks, Database, ImagePlus, Keyboard, KeyRound, Languages, ListChecks, MemoryStick, NotebookText, PenLine, RefreshCw, Wallet, Zap, MessagesSquare, Music, Palette, Puzzle, Regex, ScrollText, Search, Settings2, SlidersHorizontal, Smartphone } from "@lucide/svelte";
	import { Badge, Input } from "$shared/ui";
	import { nav, type SettingsTab } from "$app/nav.svelte";
	import { logs } from "$shared/stores/log.svelte";
	import GeneralSection from "./GeneralSection.svelte";
	import ThemeSection from "./ThemeSection.svelte";
	import ShortcutSection from "./ShortcutSection.svelte";
	import WidgetSection from "./WidgetSection.svelte";
	import ApiSection from "./ApiSection.svelte";
	import PresetSection from "./PresetSection.svelte";
	import RegexSection from "./RegexSection.svelte";
	import LorebookSection from "./LorebookSection.svelte";
	import DataSection from "./DataSection.svelte";
	import SyncSection from "./SyncSection.svelte";
	import CommunitySection from "./CommunitySection.svelte";
	import CommunityPluginSettings from "./CommunityPluginSettings.svelte";
	import { admin } from "$features/community/admin.svelte";
	import LogSection from "./LogSection.svelte";
	import ActivitySection from "./ActivitySection.svelte";
	import BudgetSection from "./BudgetSection.svelte";
	import GameSection from "./GameSection.svelte";
	import CgSection from "./CgSection.svelte";
	import ThemeEditor from "./ThemeEditor.svelte";
	import FontManagerSection from "./FontManagerSection.svelte";
	import ApiEditor from "./ApiEditor.svelte";
	import ChatSection from "./ChatSection.svelte";
	import PhoneSection from "./PhoneSection.svelte";
	import PluginsSection from "./PluginsSection.svelte";
	import TranslateSection from "./TranslateSection.svelte";
	import LintSection from "./LintSection.svelte";
	import MemorySection from "./MemorySection.svelte";
	import SummarySection from "./SummarySection.svelte";
	import RewriteSection from "./RewriteSection.svelte";
	import GhostSection from "./GhostSection.svelte";
	import QuickReplySection from "./QuickReplySection.svelte";
	import TtsSection from "./TtsSection.svelte";
	import ImageGenSection from "./ImageGenSection.svelte";
	import { plugins, type PluginId } from "$shared/stores/plugins.svelte";

	let { tab: routeTab, edit }: { tab?: SettingsTab; edit?: string } = $props();
	const tab = $derived(routeTab ?? "general");
	// 휴대폰: 탭을 고르기 전에는 메뉴 목록만, 고르면 그 화면만 (뒤로·경로로 목록에)
	const mobileList = $derived(layout.narrow && !routeTab);

	interface Item {
		id: SettingsTab;
		label: string;
		icon: typeof Settings2;
		/** 검색어로 찾을 낱말 */
		keywords: string;
		view: Component<any>;
		/** 화면에 넘길 값 (설치된 커뮤니티 플러그인 탭이 플러그인 id를 받음) */
		props?: Record<string, unknown>;
		/** 그룹의 맨 아래에 둠 (이 그룹에 항목이 늘어도 늘 마지막) */
		last?: boolean;
		/** 이 코어 플러그인이 꺼져 있으면 탭을 숨김 */
		/** 이 코어 플러그인(여럿이면 하나라도)이 꺼져 있으면 탭을 숨김 */
		plugin?: PluginId | PluginId[];
	}
	$effect(() => {
		if (!plugins.loaded) void plugins.load();
	});
	// 설치된 커뮤니티 플러그인 중 설정칸이 있는 것이 이 그룹의 탭이 된다 (코어 플러그인이 켜져 있을 때)
	$effect(() => {
		if (plugins.on("community") && !admin.loaded) void admin.load();
	});
	const communityItems = $derived<Item[]>(
		plugins.on("community")
			? admin.list
					.filter((p) => p.manifest?.settings.length)
					.map((p) => ({
						id: `community:${p.id}` as SettingsTab,
						label: p.manifest?.name ?? p.id,
						icon: Puzzle,
						keywords: $_("settings.extra.communityTabKeywords", { values: { id: p.id } }),
						view: CommunityPluginSettings,
						props: { pluginId: p.id },
					}))
			: [],
	);
	const allGroups = $derived<{ id: string; title: string; items: Item[] }[]>([
		{
			id: "app",
			title: $_("settings.groups.app"),
			items: [
				{ id: "general", label: $_("settings.tabs.general"), icon: Settings2, keywords: $_("settings.extra.tabKeywords.general"), view: GeneralSection },
				{ id: "chat", label: $_("settings.tabs.chat"), icon: MessagesSquare, keywords: $_("settings.extra.tabKeywords.chat"), view: ChatSection },
				{ id: "theme", label: $_("settings.tabs.theme"), icon: Palette, keywords: $_("settings.extra.tabKeywords.theme"), view: ThemeSection },
				{ id: "shortcut", label: $_("settings.tabs.shortcut"), icon: Keyboard, keywords: $_("settings.extra.tabKeywords.shortcut"), view: ShortcutSection },
				{ id: "plugins", label: $_("settings.tabs.plugins"), icon: Blocks, keywords: $_("settings.extra.tabKeywords.plugins"), view: PluginsSection, last: true },
				{ id: "community", label: $_("settings.tabs.community"), icon: Puzzle, keywords: $_("settings.extra.tabKeywords.community"), view: CommunitySection, last: true },
			],
		},
		{
			id: "plugins",
			title: $_("settings.groups.plugins"),
			items: [
				{ id: "game", label: $_("settings.tabs.game"), icon: Gamepad2, keywords: $_("settings.extra.tabKeywords.game"), view: GameSection, plugin: "game" },
				{ id: "cg", label: $_("settings.tabs.cg"), icon: Images, keywords: $_("settings.extra.tabKeywords.cg"), view: CgSection, plugin: "chat.images" },
				{ id: "phone", label: $_("settings.tabs.phone"), icon: Smartphone, keywords: $_("settings.extra.tabKeywords.phone"), view: PhoneSection, plugin: "phone" },
				{ id: "translate", label: $_("settings.tabs.translate"), icon: Languages, keywords: $_("settings.extra.tabKeywords.translate"), view: TranslateSection, plugin: "translate" },
				{ id: "lint", label: $_("settings.tabs.lint"), icon: ListChecks, keywords: $_("settings.extra.tabKeywords.lint"), view: LintSection, plugin: "lint" },
				{ id: "memory", label: $_("settings.tabs.memory"), icon: MemoryStick, keywords: $_("settings.extra.tabKeywords.memory"), view: MemorySection, plugin: "memory" },
				{ id: "ghostwrite", label: $_("settings.tabs.ghostwrite"), icon: PenLine, keywords: $_("settings.extra.tabKeywords.ghostwrite"), view: GhostSection, plugin: "ghostwrite" },
				{ id: "quickreply", label: $_("settings.tabs.quickreply"), icon: Zap, keywords: $_("settings.extra.tabKeywords.quickreply"), view: QuickReplySection, plugin: "slash" },
				{ id: "rewrite", label: $_("settings.tabs.rewrite"), icon: RefreshCw, keywords: $_("settings.extra.tabKeywords.rewrite"), view: RewriteSection, plugin: "rewrite" },
				{ id: "summary", label: $_("settings.tabs.summary"), icon: NotebookText, keywords: $_("settings.extra.tabKeywords.summary"), view: SummarySection, plugin: "summary" },
				{ id: "tts", label: $_("settings.tabs.tts"), icon: AudioLines, keywords: $_("settings.extra.tabKeywords.tts"), view: TtsSection, plugin: "tts" },
				{ id: "imagegen", label: $_("settings.tabs.imagegen"), icon: ImagePlus, keywords: $_("settings.extra.tabKeywords.imagegen"), view: ImageGenSection, plugin: ["imagegen", "imageauto"] },
				{ id: "widget", label: $_("settings.tabs.widget"), icon: Music, keywords: $_("settings.extra.tabKeywords.widget"), view: WidgetSection, plugin: "widget" },
			],
		},
		{
			id: "ai",
			title: $_("settings.groups.ai"),
			items: [
				{ id: "api", label: $_("settings.tabs.api"), icon: KeyRound, keywords: $_("settings.extra.tabKeywords.api"), view: ApiSection },
				{ id: "preset", label: $_("settings.tabs.preset"), icon: SlidersHorizontal, keywords: $_("settings.extra.tabKeywords.preset"), view: PresetSection },
				{ id: "lorebook", label: $_("settings.tabs.lorebook"), icon: BookOpen, keywords: $_("settings.extra.tabKeywords.lorebook"), view: LorebookSection },
				{ id: "regex", label: $_("settings.tabs.regex"), icon: Regex, keywords: $_("settings.extra.tabKeywords.regex"), view: RegexSection, plugin: "regex" },
			],
		},
		{
			id: "manage",
			title: $_("settings.groups.manage"),
			items: [
				{ id: "activity", label: $_("settings.tabs.activity"), icon: Activity, keywords: $_("settings.extra.tabKeywords.activity"), view: ActivitySection },
				{ id: "budget", label: $_("settings.tabs.budget"), icon: Wallet, keywords: $_("settings.extra.tabKeywords.budget"), view: BudgetSection },
				{ id: "data", label: $_("settings.tabs.data"), icon: Database, keywords: $_("settings.extra.tabKeywords.data"), view: DataSection },
				{ id: "sync", label: $_("settings.tabs.sync"), icon: ArrowLeftRight, keywords: $_("settings.extra.tabKeywords.sync"), view: SyncSection, plugin: "sync" },
				{ id: "log", label: $_("settings.tabs.log"), icon: ScrollText, keywords: $_("settings.extra.tabKeywords.log"), view: LogSection },
			],
		},
		{ id: "community", title: $_("settings.groups.community"), items: communityItems },
	]);
	/** 그룹 순서 */
	const ORDER = ["app", "ai", "plugins", "manage", "community"];
	/** 꺼진 코어 플러그인의 설정 탭은 숨긴다 */
	const groups = $derived([...allGroups].sort((a, b) => ORDER.indexOf(a.id) - ORDER.indexOf(b.id)).map((g) => ({ ...g, items: [...g.items.filter((i) => !i.last), ...g.items.filter((i) => i.last)].filter((i) => !i.plugin || (Array.isArray(i.plugin) ? i.plugin.some((p) => plugins.on(p)) : plugins.on(i.plugin))) })).filter((g) => g.items.length));
	const all = $derived(groups.flatMap((g) => g.items));

	let query = $state("");
	const q = $derived(query.trim().toLowerCase());
	const shown = $derived(
		q ? groups.map((g) => ({ ...g, items: g.items.filter((i) => `${i.label} ${i.keywords}`.toLowerCase().includes(q)) })).filter((g) => g.items.length) : groups,
	);
	const current = $derived(all.find((i) => i.id === tab) ?? all[0]);

	// 탭 전환도 페이지 이동 기록에 남김 (뒤로·앞으로 가기로 설정 탭 간 이동 가능)
	function select(id: SettingsTab) {
		void nav.go({ page: "settings", tab: id });
	}

	let body: HTMLDivElement | undefined = $state();
	$effect(() => {
		void current.id;
		void edit;
		body?.scrollTo({ top: 0 });
	});
</script>

<div class="settings" class:desktop={!layout.mobile} class:mobile={layout.mobile} class:narrow={layout.narrow} class:listing={mobileList}>
	<nav class="side" aria-label={$_("uiPolish.settingsNavAria")}>
		<div class="search">
			<Input bind:value={query} placeholder={$_("settings.searchPlaceholder")} aria-label={$_("settings.searchPlaceholder")} type="search">
				{#snippet leading()}<Search />{/snippet}
			</Input>
		</div>
		{#each shown as g (g.id)}
			<p class="group">{g.title}</p>
			<ul>
				{#each g.items as it (it.id)}
					<li>
						<button class:on={current.id === it.id} aria-current={current.id === it.id ? "page" : undefined} onclick={() => select(it.id)}>
							<it.icon />{it.label}
							{#if it.id === "log" && logs.errorCount}<span class="count"><Badge tone="danger">{logs.errorCount}</Badge></span>{/if}
						</button>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="none">{$_("settings.noMatch")}</p>
		{/each}
	</nav>

	<div class="body" bind:this={body}>
		<div class="inner">
			{#if current.id === "theme" && edit === "fonts"}
				<FontManagerSection />
			{:else if current.id === "theme" && edit}
				{#key edit}<ThemeEditor editKey={edit} />{/key}
			{:else if current.id === "api" && edit}
				{#key edit}<ApiEditor editId={edit} />{/key}
			{:else}
				{#key current.id}
					<current.view {...current.props ?? {}} />
				{/key}
			{/if}
		</div>
	</div>
</div>

<style>
	.settings {
		display: grid;
		grid-template-columns: 260px minmax(0, 1fr);
		grid-template-rows: minmax(0, 1fr);
		height: 100%;
	}
	.side {
		padding: var(--space-3);
		border-right: 1px solid var(--border);
		background: var(--bg);
		overflow: auto;
	}
	.search {
		margin-bottom: var(--space-3);
	}
	.group {
		margin: var(--space-4) var(--space-2) var(--space-1);
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--text-faint);
	}
	.none {
		padding: var(--space-4) var(--space-2);
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
	.side ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.side button {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		height: 30px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		cursor: pointer;
	}
	.side button > :global(svg) {
		width: 16px;
		height: 16px;
	}
	.side button:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.side button.on {
		background: var(--accent-soft);
		color: var(--text);
		font-weight: 600;
	}
	.side button.on > :global(svg) {
		color: var(--accent);
	}
	.count {
		margin-left: auto;
	}
	.body {
		overflow: auto;
		background: var(--surface-2);
	}
	.inner {
		max-width: var(--page-narrow);
		margin: 0 auto;
		padding: var(--space-8) var(--space-6);
	}
	/* 좁은 화면(휴대폰·좁힌 창): 목록과 화면을 한 번에 하나만. 고르는 흐름(mobileList)이 layout.narrow를 따르므로 클래스도 narrow로 */
	.settings.narrow {
		grid-template-columns: minmax(0, 1fr);
	}
	.settings.narrow:not(.listing) .side,
	.settings.listing .body {
		display: none;
	}
	.settings.listing .side {
		border-right: none;
	}
	/* 휴대폰 메뉴 목록: 손가락으로 누르기 쉽게 줄 높이를 --touch-target까지 */
	.settings.listing .side button {
		height: var(--touch-target);
	}
</style>
