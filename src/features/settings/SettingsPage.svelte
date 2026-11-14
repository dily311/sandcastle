<script lang="ts">
	import type { Component } from "svelte";
	import { Database, Keyboard, KeyRound, Music, Palette, Regex, ScrollText, Search, Settings2, SlidersHorizontal } from "@lucide/svelte";
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
	import LogSection from "./LogSection.svelte";

	let { tab = "general" }: { tab?: SettingsTab } = $props();

	interface Item {
		id: SettingsTab;
		label: string;
		icon: typeof Settings2;
		/** 검색어로 찾을 낱말 */
		keywords: string;
		view: Component;
	}
	const groups: { title: string; items: Item[] }[] = [
		{
			title: "앱",
			items: [
				{ id: "general", label: "기본 설정", icon: Settings2, keywords: "언어 enter 보내기 시각 업데이트 기본 api 프리셋 페르소나 폰 프롬프트", view: GeneralSection },
				{ id: "theme", label: "테마", icon: Palette, keywords: "화면 모드 다크 어둡게 밝게 강조색 글자 크기 프레임 창 css", view: ThemeSection },
				{ id: "shortcut", label: "단축키", icon: Keyboard, keywords: "키 단축 뒤로 앞으로", view: ShortcutSection },
				{ id: "widget", label: "위젯", icon: Music, keywords: "음악 플레이어 자동 실행 바탕화면", view: WidgetSection },
			],
		},
		{
			title: "AI",
			items: [
				{ id: "api", label: "API 키", icon: KeyRound, keywords: "연결 공급자 모델 openai claude gemini 키 테스트", view: ApiSection },
				{ id: "preset", label: "프리셋", icon: SlidersHorizontal, keywords: "프롬프트 블록 온도 토큰 컨텍스트 sillytavern", view: PresetSection },
				{ id: "regex", label: "정규식", icon: Regex, keywords: "찾아 바꾸기 치환 regex", view: RegexSection },
			],
		},
		{
			title: "관리",
			items: [
				{ id: "data", label: "데이터", icon: Database, keywords: "백업 복원 저장 공간 용량 삭제 정리", view: DataSection },
				{ id: "log", label: "로그", icon: ScrollText, keywords: "오류 경고 기록", view: LogSection },
			],
		},
	];
	const all = groups.flatMap((g) => g.items);

	let query = $state("");
	const q = $derived(query.trim().toLowerCase());
	const shown = $derived(
		q ? groups.map((g) => ({ ...g, items: g.items.filter((i) => `${i.label} ${i.keywords}`.toLowerCase().includes(q)) })).filter((g) => g.items.length) : groups,
	);
	const current = $derived(all.find((i) => i.id === tab) ?? all[0]);

	// 탭 전환은 기록에 남기지 않음 (뒤로 가기는 설정 전 화면으로)
	function select(id: SettingsTab) {
		nav.replace({ page: "settings", tab: id });
	}

	let body: HTMLDivElement | undefined = $state();
	$effect(() => {
		void current.id;
		body?.scrollTo({ top: 0 });
	});
</script>

<div class="settings">
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
			{#key current.id}
				<current.view />
			{/key}
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
		max-width: 760px;
		margin: 0 auto;
		padding: var(--space-8) var(--space-6);
	}
	@media (max-width: 720px) {
		.settings {
			grid-template-columns: minmax(0, 1fr);
		}
		.side {
			display: none;
		}
	}
</style>
