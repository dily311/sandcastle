<script lang="ts">
	import { layout } from "$app/layout.svelte";
	import type { Component } from "svelte";
	import { Activity, ArrowLeftRight, Gamepad2, Images, AudioLines, Blocks, Database, ImagePlus, Keyboard, KeyRound, Languages, ListChecks, MemoryStick, NotebookText, PenLine, RefreshCw, Zap, MessagesSquare, Music, Palette, Puzzle, Regex, ScrollText, Search, Settings2, SlidersHorizontal, Smartphone } from "@lucide/svelte";
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
	import DataSection from "./DataSection.svelte";
	import SyncSection from "./SyncSection.svelte";
	import CommunitySection from "./CommunitySection.svelte";
	import CommunityPluginSettings from "./CommunityPluginSettings.svelte";
	import { admin } from "$features/community/admin.svelte";
	import LogSection from "./LogSection.svelte";
	import ActivitySection from "./ActivitySection.svelte";
	import GameSection from "./GameSection.svelte";
	import CgSection from "./CgSection.svelte";
	import ThemeEditor from "./ThemeEditor.svelte";
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
						keywords: `${p.id} 커뮤니티 플러그인 설정`,
						view: CommunityPluginSettings,
						props: { pluginId: p.id },
					}))
			: [],
	);
	const allGroups = $derived<{ title: string; items: Item[] }[]>([
		{
			title: "앱",
			items: [
				{ id: "general", label: "기본 설정", icon: Settings2, keywords: "언어 업데이트", view: GeneralSection },
				{ id: "chat", label: "대화", icon: MessagesSquare, keywords: "enter 보내기 메시지 시각 시간 새 대화 기본값 api 프리셋 페르소나", view: ChatSection },
				{ id: "theme", label: "테마", icon: Palette, keywords: "화면 모드 다크 어둡게 밝게 강조색 글자 크기 프레임 창 css", view: ThemeSection },
				{ id: "shortcut", label: "단축키", icon: Keyboard, keywords: "키 단축 뒤로 앞으로", view: ShortcutSection },
				// 코어·커뮤니티 플러그인은 앱 그룹의 맨 아래 (앱 설정이 늘어도 늘 마지막, last: true). 꺼져 있어도 항상 보임
				{ id: "plugins", label: "코어 플러그인", icon: Blocks, keywords: "기능 켜기 끄기 토글 플러그인 폰 게임 위젯 정규식 이미지 html 번역 음성 tts 이미지 생성 동기화", view: PluginsSection, last: true },
				{ id: "community", label: "커뮤니티 플러그인", icon: Puzzle, keywords: "플러그인 확장 설치 권한 zip 폴더 후처리 프롬프트 패널 명령 ai 제안", view: CommunitySection, last: true },
			],
		},
		{
			title: "플러그인",
			items: [
				{ id: "game", label: "게임 모드", icon: Gamepad2, keywords: "게임 규칙 씬 서술자 npc 프롬프트", view: GameSection, plugin: "game" },
				{ id: "cg", label: "캐릭터 에셋", icon: Images, keywords: "cg 이미지 키 프롬프트 에셋", view: CgSection, plugin: "chat.images" },
				{ id: "phone", label: "폰", icon: Smartphone, keywords: "폰 모드 메신저 프롬프트", view: PhoneSection, plugin: "phone" },
				{ id: "translate", label: "번역", icon: Languages, keywords: "번역 언어 자동 번역 입력", view: TranslateSection, plugin: "translate" },
				{ id: "lint", label: "메시지 linter", icon: ListChecks, keywords: "linter 검사 후처리 태그 따옴표 반복 금지어 길이 고침", view: LintSection, plugin: "lint" },
				{ id: "memory", label: "메모리 검색", icon: MemoryStick, keywords: "메모리 임베딩 검색 뜻으로 찾기 색인 벡터 기억", view: MemorySection, plugin: "memory" },
				{ id: "ghostwrite", label: "대필", icon: PenLine, keywords: "대필 초안 대신 쓰기 내 말투 펜 입력", view: GhostSection, plugin: "ghostwrite" },
				{ id: "quickreply", label: "빠른 답장", icon: Zap, keywords: "빠른 답장 qr 슬래시 커맨드 스크립트 gen input 버튼", view: QuickReplySection, plugin: "slash" },
				{ id: "rewrite", label: "자동 재생성", icon: RefreshCw, keywords: "재생성 다시 쓰기 금지어 반복 linter 스와이프", view: RewriteSection, plugin: "rewrite" },
				{ id: "summary", label: "대화 요약", icon: NotebookText, keywords: "요약 줄거리 메모리 확장 기억 긴 대화", view: SummarySection, plugin: "summary" },
				{ id: "tts", label: "음성 읽기", icon: AudioLines, keywords: "음성 tts 읽기 목소리 elevenlabs", view: TtsSection, plugin: "tts" },
				{ id: "imagegen", label: "이미지 생성", icon: ImagePlus, keywords: "이미지 그림 생성 novelai 프롬프트 네거티브 샘플러", view: ImageGenSection, plugin: ["imagegen", "imageauto"] },
				{ id: "widget", label: "위젯", icon: Music, keywords: "음악 플레이어 자동 실행 바탕화면", view: WidgetSection, plugin: "widget" },
			],
		},
		{
			title: "AI",
			items: [
				{ id: "api", label: "API 키", icon: KeyRound, keywords: "연결 공급자 모델 openai claude gemini vertex 서비스 계정 json 키 테스트", view: ApiSection },
				{ id: "preset", label: "프리셋", icon: SlidersHorizontal, keywords: "프롬프트 블록 온도 토큰 컨텍스트 sillytavern", view: PresetSection },
				{ id: "regex", label: "정규식", icon: Regex, keywords: "찾아 바꾸기 치환 regex", view: RegexSection, plugin: "regex" },
			],
		},
		{
			title: "관리",
			items: [
				{ id: "activity", label: "실행 기록", icon: Activity, keywords: "작업 사용량 토큰 비용 요금 api 실행 중 멈추기 취소 자동 기록", view: ActivitySection },
				{ id: "data", label: "데이터", icon: Database, keywords: "백업 복원 저장 공간 용량 삭제 정리 휴지통 되살리기 점검", view: DataSection },
				{ id: "sync", label: "동기화", icon: ArrowLeftRight, keywords: "기기 폰 pc 옮기기 합치기 동기화 내보내기 가져오기 충돌", view: SyncSection, plugin: "sync" },
				{ id: "log", label: "로그", icon: ScrollText, keywords: "오류 경고 기록", view: LogSection },
			],
		},
		{ title: "커뮤니티 플러그인", items: communityItems },
	]);
	/** 그룹 순서 */
	const ORDER = ["앱", "AI", "플러그인", "관리", "커뮤니티 플러그인"];
	/** 꺼진 코어 플러그인의 설정 탭은 숨긴다 */
	const groups = $derived([...allGroups].sort((a, b) => ORDER.indexOf(a.title) - ORDER.indexOf(b.title)).map((g) => ({ ...g, items: [...g.items.filter((i) => !i.last), ...g.items.filter((i) => i.last)].filter((i) => !i.plugin || (Array.isArray(i.plugin) ? i.plugin.some((p) => plugins.on(p)) : plugins.on(i.plugin))) })).filter((g) => g.items.length));
	const all = $derived(groups.flatMap((g) => g.items));

	let query = $state("");
	const q = $derived(query.trim().toLowerCase());
	const shown = $derived(
		q ? groups.map((g) => ({ ...g, items: g.items.filter((i) => `${i.label} ${i.keywords}`.toLowerCase().includes(q)) })).filter((g) => g.items.length) : groups,
	);
	const current = $derived(all.find((i) => i.id === tab) ?? all[0]);

	// 탭 전환은 기록에 남기지 않음 (뒤로 가기는 설정 전 화면으로)
	function select(id: SettingsTab) {
		if (layout.narrow) return void nav.go({ page: "settings", tab: id });
		nav.replace({ page: "settings", tab: id });
	}

	let body: HTMLDivElement | undefined = $state();
	$effect(() => {
		void current.id;
		void edit;
		body?.scrollTo({ top: 0 });
	});
</script>

<div class="settings" class:desktop={!layout.mobile} class:mobile={layout.mobile} class:narrow={layout.narrow} class:listing={mobileList}>
	<nav class="side" aria-label="설정 메뉴">
		<div class="search">
			<Input bind:value={query} placeholder="설정 검색" aria-label="설정 검색" type="search">
				{#snippet leading()}<Search />{/snippet}
			</Input>
		</div>
		{#each shown as g (g.title)}
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
			<p class="none">맞는 설정 없음</p>
		{/each}
	</nav>

	<div class="body" bind:this={body}>
		<div class="inner">
			{#if current.id === "theme" && edit}
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
		grid-template-columns: 240px minmax(0, 1fr);
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
		background: var(--surface-active);
		color: var(--text);
		font-weight: 600;
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
