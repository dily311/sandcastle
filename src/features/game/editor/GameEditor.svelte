<script lang="ts">
	import { onMount, tick } from "svelte";
	import {
		AlertTriangle,
		ChevronLeft,
		Clapperboard,
		CloudSun,
		Clock3,
		Image,
		Map as MapIcon,
		Settings2,
		Sparkles,
		Users,
		Variable,
		X,
	} from "@lucide/svelte";
	import type { PackageData } from "$features/game/types";
	import type { Issue } from "$ipc/bindings";
	import { appDataDir, join } from "@tauri-apps/api/path";
	import { Badge, Button, EmptyState, Field, IconButton, Input, Spinner, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { nav } from "$app/nav.svelte";
	import { games } from "../game.svelte";
	import "./editor.css";
	import SceneEditorSection from "./SceneEditorSection.svelte";
	import StateDefEditorSection from "./StateDefEditorSection.svelte";
	import MapEditorSection from "./MapEditorSection.svelte";
	import CharacterEditorSection from "./CharacterEditorSection.svelte";
	import WeatherEditorSection from "./WeatherEditorSection.svelte";
	import TimePeriodEditorSection from "./TimePeriodEditorSection.svelte";
	import EventEditorSection from "./EventEditorSection.svelte";
	import AssetEditorSection from "./AssetEditorSection.svelte";

	let { gameId }: { gameId: string } = $props();
	const src = { source: "game" };

	type Tab = "project" | "scenes" | "stateDefs" | "characters" | "maps" | "weather" | "time" | "event" | "assets";
	let tab = $state<Tab>("project");
	let data = $state<PackageData | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let assetRoot = $state("");
	let original = $state("");
	let issues = $state<Issue[]>([]);
	let issuesOpen = $state(false);

	async function load() {
		loading = true;
		try {
			data = (await call(commands.getPackageData(gameId), src)) as unknown as PackageData;
			original = snap(data);
			lastTab = null;
		} catch {
			data = null;
		} finally {
			loading = false;
		}
	}
	onMount(async () => {
		await load();
		try {
			assetRoot = await join(await appDataDir(), "games", gameId);
		} catch {
			/* 브라우저 개발 모드: 미리보기 없음 */
		}
	});

	/** 비교용: 없음·null·빈 문자열·빈 배열을 같게 (입력칸이 빈 값을 채워 넣는 것까지 바뀐 걸로 치지 않게) */
	const snap = (d: unknown) => JSON.stringify(d, (_k, v) => (v === null || v === "" || (Array.isArray(v) && v.length === 0) ? undefined : v));
	const dirty = $derived(!!data && snap(data) !== original);
	// 섹션은 처음 열릴 때 빠진 기본값을 채워 넣는다 → 사용자가 고친 게 없으면 그 상태를 기준으로 다시 잡음
	let lastTab: Tab | null = null;
	$effect(() => {
		const t = tab;
		if (!data || t === lastTab) return;
		const clean = snap(data) === original || lastTab === null;
		lastTab = t;
		if (clean) void tick().then(() => setTimeout(() => (original = snap(data)), 60));
	});
	$effect(() => {
		if (!dirty) return;
		return nav.setGuard(() => confirmAction({ title: "저장하지 않음", message: "바뀐 게임 데이터를 버리고 나감", confirmLabel: "버리기" }));
	});

	async function save() {
		if (!data || saving) return;
		saving = true;
		try {
			const report = await call(commands.savePackage(gameId, $state.snapshot(data) as never), src);
			issues = report.diagnostics;
			const removed = Object.values(report.deleted).reduce((a, b) => a + b, 0);
			await load();
			// 목록·상세에 보이는 이름 등도 맞춤
			await games.load();
			if (games.opened?.project.id === gameId) games.opened = null;
			const errors = issues.filter((i) => i.severity === "error").length;
			if (errors) {
				issuesOpen = true;
				toasts.error(`저장함 · 오류 ${errors}개가 있어 새 게임을 시작할 수 없음`);
			} else toasts.success(`저장함${removed ? ` · ${removed}개 지움` : ""}${issues.length ? ` · 경고 ${issues.length}개` : ""}`);
		} finally {
			saving = false;
		}
	}
	function back() {
		void nav.replace({ page: "game", gameId });
	}
	function onKey(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
			e.preventDefault();
			void save();
		}
	}

	const assetCount = $derived(
		data ? data.assets.length + data.audioAssets.length + data.characterAssets.length + data.cgAssets.length + data.backgroundAssets.length : 0,
	);
	const tabs = $derived<{ id: Tab; label: string; icon: typeof Users; count?: number; hint: string }[]>(
		data
			? [
					{ id: "project", label: "기본 정보", icon: Settings2, hint: "제목·설명·사용자 CSS" },
					{ id: "scenes", label: "씬", icon: Clapperboard, count: data.scenes.length, hint: "대본과 선택지" },
					{ id: "characters", label: "등장인물", icon: Users, count: data.characters.length, hint: "인물·표정 이미지" },
					{ id: "maps", label: "지도", icon: MapIcon, count: data.locations.length, hint: "지도·지역·장소·이동 경로" },
					{ id: "event", label: "이벤트", icon: Sparkles, count: data.events.length, hint: "조건이 맞으면 일어나는 일" },
					{ id: "stateDefs", label: "변수", icon: Variable, count: data.stateDefs.length, hint: "게임 상태 값" },
					{ id: "time", label: "시간대", icon: Clock3, count: data.timePeriods.length, hint: "아침·낮·밤" },
					{ id: "weather", label: "날씨", icon: CloudSun, count: data.weatherTypes.length, hint: "날씨와 바뀌는 규칙" },
					{ id: "assets", label: "에셋", icon: Image, count: assetCount, hint: "이미지·소리·배경" },
				]
			: [],
	);
	const errors = $derived(issues.filter((i) => i.severity === "error"));
</script>

<svelte:window onkeydown={onKey} />

{#if loading}
	<div class="center"><Spinner /></div>
{:else if !data}
	<EmptyState title="게임 데이터를 불러올 수 없음" description="파일이 깨졌거나 지워졌을 수 있음">
		{#snippet action()}<Button onclick={back}>게임으로</Button>{/snippet}
	</EmptyState>
{:else}
	<div class="ge">

		{#if issuesOpen && issues.length}
			<div class="issues">
				<div class="issues-head"><b>저장 후 검사 결과</b><IconButton label="닫기" size="sm" onclick={() => (issuesOpen = false)}><X /></IconButton></div>
				<ul>
					{#each issues as d, i (i)}
						<li><Badge tone={d.severity === "error" ? "danger" : "warning"}>{d.severity === "error" ? "오류" : "경고"}</Badge>{d.text}</li>
					{/each}
				</ul>
			</div>
		{/if}

		<div class="layout">
			<nav class="side" aria-label="편집 항목">
				{#each tabs as t (t.id)}
					<button class:on={tab === t.id} aria-current={tab === t.id ? "page" : undefined} onclick={() => (tab = t.id)}>
						<t.icon /><span>{t.label}</span>{#if t.count !== undefined}<small>{t.count}</small>{/if}
					</button>
				{/each}
				<div class="bottom">
					{#if issues.length}
						<button class="issues-btn" class:err={errors.length} aria-expanded={issuesOpen} onclick={() => (issuesOpen = !issuesOpen)}>
							<AlertTriangle />{errors.length ? `오류 ${errors.length}` : `경고 ${issues.length}`}
						</button>
					{/if}
					{#if dirty}<span class="dirty">바뀐 내용 있음</span>{/if}
					<Button variant="primary" loading={saving} disabled={!dirty} onclick={save}>저장 <kbd>Ctrl S</kbd></Button>
				</div>
			</nav>

			<main class="body game-editor" class:wide={tab !== "project"}>
				{#if tab === "project"}
					<div class="project">
						<h2>기본 정보</h2>
						<div class="card">
							<Field label="제목" required><Input bind:value={data.project.title} /></Field>
							<Field label="설명" hint="게임 목록과 상세에 보임"><Textarea bind:value={data.project.description} rows={3} autosize maxRows={10} /></Field>
						</div>
						<h2>사용자 CSS</h2>
						<div class="card">
							<Field label="게임 화면에만 적용되는 CSS" hint="고급 · 비워 두면 기본 모양">
								<Textarea bind:value={data.project.customCss} mono rows={8} spellcheck={false} />
							</Field>
						</div>
					</div>
				{:else if tab === "scenes"}
					<SceneEditorSection bind:scenes={data.scenes} gameId={data.project.id} />
				{:else if tab === "stateDefs"}
					<StateDefEditorSection bind:stateDefs={data.stateDefs} gameId={data.project.id} />
				{:else if tab === "characters"}
					<CharacterEditorSection
						bind:characters={data.characters}
						bind:assets={data.characterAssets}
						availableImages={[...data.assets, ...data.characterAssets, ...data.cgAssets, ...data.backgroundAssets].filter((asset) => asset.kind === "image" && !!asset.filePath)}
						gameId={data.project.id}
						{assetRoot}
					/>
				{:else if tab === "maps"}
					<MapEditorSection
						bind:zones={data.zones}
						bind:maps={data.maps}
						bind:locations={data.locations}
						bind:backgroundAssets={data.backgroundAssets}
						bind:connections={data.locationConnections}
						bind:interactables={data.interactables}
						events={data.events}
						timePeriods={data.timePeriods}
						weatherTypes={data.weatherTypes}
						gameId={data.project.id}
						{assetRoot}
					/>
				{:else if tab === "weather"}
					<WeatherEditorSection bind:weatherTypes={data.weatherTypes} bind:weatherTransitions={data.weatherTransitions} maps={data.maps} gameId={data.project.id} />
				{:else if tab === "time"}
					<TimePeriodEditorSection bind:timePeriods={data.timePeriods} gameId={data.project.id} />
				{:else if tab === "event"}
					<EventEditorSection bind:events={data.events} items={data.items} stateDefs={data.stateDefs} scenes={data.scenes} characters={data.characters} gameId={data.project.id} />
				{:else if tab === "assets"}
					<AssetEditorSection
						bind:assets={data.assets}
						bind:audioAssets={data.audioAssets}
						bind:characterAssets={data.characterAssets}
						bind:cgAssets={data.cgAssets}
						bind:backgroundAssets={data.backgroundAssets}
						characters={data.characters}
						locations={data.locations}
						timePeriods={data.timePeriods}
						weatherTypes={data.weatherTypes}
						gameId={data.project.id}
						{assetRoot}
					/>
				{/if}
			</main>
		</div>
	</div>
{/if}

<style>
	.center {
		display: grid;
		place-items: center;
		height: 100%;
	}
	.ge {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}
	.top {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-4) var(--space-2) var(--space-2);
		border-bottom: 1px solid var(--border);
		background: var(--bg);
	}
	.dirty {
		font-size: var(--text-sm);
		color: var(--accent-text);
	}
	kbd {
		margin-left: 4px;
		padding: 0 4px;
		border-radius: 4px;
		background: rgb(255 255 255 / 0.2);
		font-family: var(--font-sans);
		font-size: 10px;
	}
	.issues-btn {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 28px;
		padding: 0 10px;
		border: 1px solid var(--warning);
		border-radius: 999px;
		background: var(--warning-soft);
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		cursor: pointer;
	}
	.issues-btn.err {
		border-color: var(--danger);
		background: var(--danger-soft);
	}
	.issues-btn :global(svg) {
		width: 14px;
		height: 14px;
	}
	.issues {
		max-height: 30vh;
		overflow: auto;
		padding: var(--space-2) var(--space-4) var(--space-3);
		border-bottom: 1px solid var(--border);
		background: var(--surface-2);
	}
	.issues-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.issues ul {
		list-style: none;
		margin: var(--space-1) 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: var(--text-sm);
	}
	.issues li {
		display: flex;
		gap: var(--space-2);
		align-items: flex-start;
	}
	.layout {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: 200px minmax(0, 1fr);
	}
	.side {
		display: flex;
		flex-direction: column;
		gap: 1px;
		padding: var(--space-3) var(--space-2);
		border-right: 1px solid var(--border);
		background: var(--bg);
		overflow: auto;
	}
	.side button {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		height: 32px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.side button :global(svg) {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
	}
	.side button span {
		flex: 1;
	}
	.side button small {
		color: var(--text-faint);
		font-size: var(--text-xs);
		font-variant-numeric: tabular-nums;
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
	.side button.on :global(svg) {
		color: var(--accent);
	}
	.body {
		min-height: 0;
		overflow: auto;
		background: var(--surface-2);
	}
	.body.wide {
		overflow: hidden;
	}
	.project {
		max-width: 760px;
		margin: 0 auto;
		padding: var(--space-6);
	}
	.project h2 {
		margin: var(--space-6) 0 var(--space-3);
		font-size: var(--text-lg);
	}
	.project h2:first-child {
		margin-top: 0;
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-5);
		border-radius: var(--radius-lg);
		background: var(--bg-sunken);
	}

	.bottom {
		display: flex;
		justify-content: space-between;
		margin-top: auto;
		padding-top: var(--space-2);
		border-top: 1px solid var(--border);
	}

	@media (max-width: 860px) {
		.layout {
			grid-template-columns: minmax(0, 1fr);
		}
		.side {
			flex-direction: row;
			overflow-x: auto;
			border-right: none;
			border-bottom: 1px solid var(--border);
		}
	}
</style>
