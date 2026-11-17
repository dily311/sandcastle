<script lang="ts">
	import { layout } from "$app/layout.svelte";
	import type { Component } from "svelte";
	import { Activity, ArrowLeftRight, Gamepad2, Images, AudioLines, Blocks, Database, ImagePlus, Keyboard, KeyRound, Languages, MessagesSquare, Music, Palette, Regex, ScrollText, Search, Settings2, SlidersHorizontal, Smartphone } from "@lucide/svelte";
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
	import TtsSection from "./TtsSection.svelte";
	import ImageGenSection from "./ImageGenSection.svelte";
	import { plugins, type PluginId } from "$shared/stores/plugins.svelte";

	let { tab: routeTab, edit }: { tab?: SettingsTab; edit?: string } = $props();
	const tab = $derived(routeTab ?? "general");
	// 휴대폰: 탭을 고르기 전에는 메뉴 목록만, 고르면 그 화면만 (뒤로·경로로 목록에)
	const mobileList = $derived(layout.mobile && !routeTab);

	interface Item {
		id: SettingsTab;
		label: string;
		icon: typeof Settings2;
		/** 검색어로 찾을 낱말 */
		keywords: string;
		view: Component;
		/** 이 코어 플러그인이 꺼져 있으면 탭을 숨김 */
		/** 이 코어 플러그인(여럿이면 하나라도)이 꺼져 있으면 탭을 숨김 */
		plugin?: PluginId | PluginId[];
	}
	$effect(() => {
		if (!plugins.loaded) void plugins.load();
	});
	const allGroups: { title: string; items: Item[] }[] = [
		{
			title: "앱",
			items: [
				{ id: "general", label: "기본 설정", icon: Settings2, keywords: "언어 업데이트", view: GeneralSection },
				{ id: "chat", label: "대화", icon: MessagesSquare, keywords: "enter 보내기 메시지 시각 시간 새 대화 기본값 api 프리셋 페르소나", view: ChatSection },
				{ id: "theme", label: "테마", icon: Palette, keywords: "화면 모드 다크 어둡게 밝게 강조색 글자 크기 프레임 창 css", view: ThemeSection },
				{ id: "shortcut", label: "단축키", icon: Keyboard, keywords: "키 단축 뒤로 앞으로", view: ShortcutSection },
			],
		},
		{
			title: "플러그인",
			items: [
				{ id: "plugins", label: "코어 플러그인", icon: Blocks, keywords: "기능 켜기 끄기 토글 플러그인 폰 게임 위젯 정규식 이미지 html 번역 음성 tts 이미지 생성 동기화", view: PluginsSection },
				{ id: "game", label: "게임 모드", icon: Gamepad2, keywords: "게임 규칙 씬 서술자 npc 프롬프트", view: GameSection, plugin: "game" },
				{ id: "cg", label: "캐릭터 이미지", icon: Images, keywords: "cg 이미지 키 프롬프트 에셋", view: CgSection, plugin: "chat.images" },
				{ id: "phone", label: "폰", icon: Smartphone, keywords: "폰 모드 메신저 프롬프트", view: PhoneSection, plugin: "phone" },
				{ id: "translate", label: "번역", icon: Languages, keywords: "번역 언어 자동 번역 입력", view: TranslateSection, plugin: "translate" },
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
	];
	/** 꺼진 코어 플러그인의 설정 탭은 숨긴다 */
	const groups = $derived(allGroups.map((g) => ({ ...g, items: g.items.filter((i) => !i.plugin || (Array.isArray(i.plugin) ? i.plugin.some((p) => plugins.on(p)) : plugins.on(i.plugin))) })).filter((g) => g.items.length));
	const all = $derived(groups.flatMap((g) => g.items));

	let query = $state("");
	const q = $derived(query.trim().toLowerCase());
	const shown = $derived(
		q ? groups.map((g) => ({ ...g, items: g.items.filter((i) => `${i.label} ${i.keywords}`.toLowerCase().includes(q)) })).filter((g) => g.items.length) : groups,
	);
	const current = $derived(all.find((i) => i.id === tab) ?? all[0]);

	// 탭 전환은 기록에 남기지 않음 (뒤로 가기는 설정 전 화면으로)
	function select(id: SettingsTab) {
		if (layout.mobile) return void nav.go({ page: "settings", tab: id });
		nav.replace({ page: "settings", tab: id });
	}

	let body: HTMLDivElement | undefined = $state();
	$effect(() => {
		void current.id;
		void edit;
		body?.scrollTo({ top: 0 });
	});
</script>

<div class="settings" class:mobile={layout.mobile} class:listing={mobileList}>
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
					<current.view />
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
	.settings.mobile {
		grid-template-columns: minmax(0, 1fr);
	}
	.settings.mobile:not(.listing) .side,
	.settings.listing .body {
		display: none;
	}
	.settings.listing .side {
		border-right: none;
	}
</style>
