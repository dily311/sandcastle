<script lang="ts">
	// 게임 패널의 공통 탭 (대화·폰과 같은 자리): 대화 설정 / 컨텍스트·프리셋 / 분기
	// 게임은 대화방 대신 '세션(플레이 기록)' 단위라서 설정·분기도 세션에 붙는다.
	import { Bookmark, BookmarkPlus, CornerDownRight, GitBranch, Play } from "@lucide/svelte";
	import type { ContextUsage, GamePlayerSessions_Serialize as Session, GameSave_Serialize as Save, GameSessionOptions_Serialize as Options, TurnPoint } from "$ipc/bindings";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { gameStore } from "../runtime/gameStore.svelte";
	import { Button, IconButton, Select, ProfilePicker, type ProfileOption } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
	import { panel } from "$app/panel.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";
	import { relative } from "$shared/time";
	import { library } from "$features/character/library.svelte";
	import ContextMeter from "$features/character/ContextMeter.svelte";
	import PresetInline from "$features/settings/PresetInline.svelte";
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { runtimeStore } from "../runtime/runtimeStore.svelte";
	import { formatGameTime } from "./vn/playUtils";

	let { gameId, sessionId }: { gameId: string; sessionId: string } = $props();
	const src = { source: "game" };

	$effect(() => {
		if (!library.loaded) void library.load();
	});

	// ---- 대화 설정: 세션의 API · 프리셋 · 플레이어 ----
	let options = $state<Options | null>(null);
	async function loadOptions() {
		options = await call(commands.getGameSessionOptions(sessionId), { ...src, silent: true }).catch(() => null);
	}
	$effect(() => {
		void sessionId;
		void loadOptions();
	});
	const DEFAULT = "";
	const apiOptions = $derived([{ value: DEFAULT, label: "앱 기본값" }, ...library.apis.map((a) => ({ value: a.id, label: `${a.name} · ${a.model}` }))]);
	const presetOptions = $derived([{ value: DEFAULT, label: "앱 기본값" }, ...library.presets.map((p) => ({ value: p.id, label: p.name }))]);
	const personaOptions = $derived<ProfileOption[]>(library.personas.map((p) => ({ id: p.id, name: p.name, avatar: p.avatar, color: p.color, description: p.description })));
	async function setOption(p: Partial<{ apiProfileId: string; promptPresetId: string; playerId: string }>, model?: string | null) {
		if (!options) return;
		const next = { ...options, ...p };
		// 모델은 API에 딸린 이름: API가 바뀌면 백엔드가 초기화한다 (ADR-020). 모델을 안 바꾸는 저장은 지금 모델을 그대로 보냄
		const nextModel = model !== undefined ? model : (options.model ?? null);
		await call(commands.setGameSessionOptions(sessionId, next.apiProfileId || null, next.promptPresetId || null, p.playerId ?? null, nextModel), src);
		const apiChanged = (p.apiProfileId ?? options.apiProfileId ?? "") !== (options.apiProfileId ?? "") && !!options.apiProfileId;
		options = { ...next, apiProfileId: next.apiProfileId || null, promptPresetId: next.promptPresetId || null, model: apiChanged ? null : nextModel };
		void loadUsage();
	}
	// 이 플레이가 쓰는 API 프로필 (정한 것, 없으면 앱 기본값) — 모델 고르기가 그 프로필의 목록을 보여준다
	const effectiveApi = $derived(library.apis.find((a) => a.id === (options?.apiProfileId || appSetting.value?.selectedApiProfile || "")));
	/** 모델을 고르면 그 API도 이 플레이에 못 박는다 (앱 기본 API가 나중에 바뀌어도 모델이 엉뚱한 공급자로 가지 않게) */
	function pickModel(model: string | null) {
		if (!options?.apiProfileId && effectiveApi) return setOption({ apiProfileId: effectiveApi.id }, model);
		return setOption({}, model);
	}

	// ---- 컨텍스트: 이번에 앱을 켠 뒤 마지막으로 보낸 프롬프트 ----
	let usage = $state<ContextUsage | null>(null);
	async function loadUsage() {
		usage = await call(commands.gameContextUsage(sessionId), { ...src, silent: true }).catch(() => null);
	}
	$effect(() => {
		void sessionId;
		void runtimeStore.value; // 한 턴이 끝날 때마다
		void loadUsage();
	});
	const activePresetId = $derived(options?.promptPresetId || usage?.presetId || null);

	// ---- 분기: 세션 나무 + 책갈피 ----
	let sessions = $state<Session[]>([]);
	let saves = $state<Save[]>([]);
	/** 이 플레이의 턴 시점들 (최신 → 오래된 순). 아무 시점에서나 갈라질 수 있다 */
	let points = $state<TurnPoint[]>([]);
	async function loadBranches() {
		const [s, v, p] = await Promise.all([
			call(commands.listSessions(), { ...src, silent: true }).catch(() => []),
			call(commands.saveList(), { ...src, silent: true }).catch(() => []),
			call(commands.listTurnPoints(sessionId, 200), { ...src, silent: true }).catch(() => []),
		]);
		sessions = s;
		saves = v;
		points = p;
	}
	$effect(() => {
		void runtimeStore.value; // 한 턴이 끝날 때마다
		if (panel.tab === "branches") void loadBranches();
	});
	const sceneName = (id?: string | null) => (id ? gameStore.scenes?.find((s) => s.id === id)?.name : null) ?? null;
	const placeName = (id?: string | null) => (id ? gameStore.view?.locations.find((l) => l.id === id)?.name : null) ?? null;
	/** 시점에서 갈라지기: 그 턴이 끝난 때로 돌아간 새 플레이를 만든다 (지금 플레이는 그대로) */
	async function forkAt(p: TurnPoint) {
		const ok = await confirmAction({ title: "이 시점에서 갈라지기", message: `「${p.summary}」 뒤로 새 갈래를 만들어 이어서 플레이함. 지금 플레이는 그대로 남음`, confirmLabel: "갈라지기" });
		if (!ok) return;
		busy = true;
		try {
			const r = await call(commands.forkAt(sessionId, p.seq, null), src);
			toasts.success("새 갈래를 만들었음");
			void nav.go({ page: "game", gameId, sessionId: r.state.sessionId });
		} finally {
			busy = false;
		}
	}
	interface Node {
		s: Session;
		depth: number;
	}
	/** 지금 세션이 속한 갈래를 맨 위 조상부터 깊이 순으로 */
	const tree = $derived.by((): Node[] => {
		const byId = new Map(sessions.map((s) => [s.id, s]));
		let top = byId.get(sessionId);
		for (let i = 0; top?.parentSessionId && i < 50; i++) top = byId.get(top.parentSessionId) ?? top;
		if (!top) return [];
		const out: Node[] = [];
		const walk = (s: Session, depth: number) => {
			out.push({ s, depth });
			sessions
				.filter((c) => c.parentSessionId === s.id)
				.sort((a, b) => (a.forkedFromSeq ?? 0) - (b.forkedFromSeq ?? 0))
				.forEach((c) => walk(c, depth + 1));
		};
		walk(top, 0);
		return out;
	});
	const savesOf = (id: string) => saves.filter((v) => v.sessionId === id).sort((a, b) => a.slotNumber - b.slotNumber);
	let busy = $state(false);
	async function bookmark() {
		const used = new Set(saves.filter((v) => v.sessionId === sessionId).map((v) => v.slotNumber));
		const slot = Array.from({ length: 20 }, (_, i) => i + 1).find((n) => !used.has(n));
		if (!slot) return toasts.info("책갈피는 세션마다 20개까지. 지난 책갈피를 덮어쓰려면 게임 메뉴에서");
		busy = true;
		try {
			await call(commands.saveWrite(slot, {}), src);
			toasts.success(`책갈피 ${slot} 남김`);
			await loadBranches();
		} finally {
			busy = false;
		}
	}
	/** 책갈피에서 이어하기: 그 뒤로 기록이 있으면 새 갈래(세션)가 생김 */
	async function resume(v: Save) {
		busy = true;
		try {
			const r = await call(commands.loadSave(v.sessionId, v.slotNumber, null), src);
			const id = r.state.sessionId;
			if (id !== v.sessionId) toasts.success("책갈피에서 새 갈래를 만들었음");
			void nav.go({ page: "game", gameId, sessionId: id });
		} finally {
			busy = false;
		}
	}
</script>

{#if panel.tab === "settings"}
	<div class="panel-body">
		<section>
			<h4>이 플레이 <span class="faint">NPC 대화와 씬 서술에 씀</span></h4>
			{#if options}
				<label class="row"><span>API</span><Select value={options.apiProfileId ?? DEFAULT} options={apiOptions} onchange={(e) => setOption({ apiProfileId: e.currentTarget.value })} /></label>
				<div class="row"><span>모델</span><ModelPicker profile={effectiveApi} value={options.model} onchange={pickModel} label="이 플레이의 모델" /></div>
				<label class="row"><span>프리셋</span><Select value={options.promptPresetId ?? DEFAULT} options={presetOptions} onchange={(e) => setOption({ promptPresetId: e.currentTarget.value })} /></label>
				<div class="row">
					<span>플레이어</span>
					<ProfilePicker label="플레이어 바꾸기" options={personaOptions} value={options.playerId} onchange={(id) => setOption({ playerId: id })} />
				</div>
				<p class="hint">플레이어는 이 플레이에서 내가 맡는 페르소나. 바꾸면 다음 대사부터 반영</p>
			{:else}
				<p class="hint">불러오는 중…</p>
			{/if}
		</section>
	</div>
{:else if panel.tab === "context"}
	<div class="panel-body preset-edit">
		<section>
			<h4>컨텍스트</h4>
			<ContextMeter {usage} />
		</section>
		<section>
			<h4>프리셋 <span class="faint">{options?.promptPresetId ? "이 플레이에서 고름" : "앱 기본값"}</span></h4>
			<Select value={options?.promptPresetId ?? DEFAULT} options={presetOptions} aria-label="이 플레이의 프리셋" onchange={(e) => setOption({ promptPresetId: e.currentTarget.value })} />
		</section>
		<PresetInline presetId={activePresetId} onsaved={loadUsage} model={options?.model || effectiveApi?.model || null} />
	</div>
{:else}
	<div class="panel-body">
		<section>
			<h4>분기 <span class="faint">플레이 {tree.length}개</span></h4>
			<Button size="sm" loading={busy} onclick={bookmark}>{#snippet icon()}<BookmarkPlus />{/snippet}지금 책갈피 남기기</Button>
			<p class="hint">책갈피에서 이어하면, 그 뒤로 기록이 있을 때 원래 플레이는 두고 새 갈래가 생김</p>
			<ul class="tree">
				{#each tree as n (n.s.id)}
					{@const here = n.s.id === sessionId}
					<li style:--depth={n.depth}>
						<div class="sess" class:here>
							<GitBranch />
							<span class="t">
								<b>{n.depth === 0 ? "처음 플레이" : `#${n.s.forkedFromSeq}에서 갈라짐`}{#if here}<em>지금</em>{/if}</b>
								<small>{formatGameTime(n.s.gameTime)} · 기록 {n.s.headSeq} · {relative(n.s.updatedAt)}</small>
							</span>
							{#if !here}<IconButton label="이 플레이로" size="sm" onclick={() => nav.go({ page: "game", gameId, sessionId: n.s.id })}><Play /></IconButton>{/if}
						</div>
						{#each savesOf(n.s.id) as v (v.slotNumber)}
							<div class="save">
								<Bookmark />
								<span class="t"><b>책갈피 {v.slotNumber}{v.name ? ` · ${v.name}` : ""}</b><small>#{v.seq} · {formatGameTime(v.gameTime)} · {relative(v.updatedAt)}</small></span>
								<Button size="sm" variant="ghost" disabled={busy} onclick={() => resume(v)}>{v.seq < n.s.headSeq ? "여기서 갈라지기" : "이어하기"}</Button>
							</div>
						{/each}
					</li>
				{:else}
					<li class="hint">불러오는 중…</li>
				{/each}
			</ul>
		</section>
		<section>
			<h4>시점 <span class="faint">이 플레이의 턴 {points.length}개</span></h4>
			<p class="hint">턴이 끝날 때마다 시점이 남음. 고른 시점으로 돌아가 새 갈래를 만들 수 있음</p>
			<ol class="points">
				{#each points as p, i (p.seq)}
					<li class="point" class:now={i === 0}>
						<CornerDownRight />
						<span class="t">
							<b>{p.summary}{#if i === 0}<em>지금</em>{/if}</b>
							<small>#{p.seq} · {formatGameTime(p.gameTime)}{placeName(p.locationId) ? ` · ${placeName(p.locationId)}` : ""}{sceneName(p.sceneId) ? ` · ${sceneName(p.sceneId)}` : ""}</small>
						</span>
						{#if i > 0}<Button size="sm" variant="ghost" disabled={busy} onclick={() => forkAt(p)}>여기서 갈라지기</Button>{/if}
					</li>
				{:else}
					<li class="hint">아직 시점 없음 (이 기능 전에 진행한 턴은 목록에 없음)</li>
				{/each}
			</ol>
		</section>
	</div>
{/if}

<style>
	.row {
		display: grid;
		grid-template-columns: 64px minmax(0, 1fr);
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.hint {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.tree {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.tree li {
		margin-left: calc(var(--depth) * 14px);
		padding-left: calc(min(var(--depth), 1) * 10px);
		border-left: calc(min(var(--depth), 1) * 1px) solid var(--border);
	}
	.points {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.point {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 6px var(--space-2);
		border-radius: var(--radius-sm);
		font-size: var(--text-sm);
	}
	.point.now {
		background: var(--accent-soft);
	}
	.point > :global(svg) {
		flex: none;
		width: 14px;
		height: 14px;
		color: var(--text-muted);
	}
	.sess,
	.save {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 6px var(--space-2);
		border-radius: var(--radius-sm);
		font-size: var(--text-sm);
	}
	.sess.here {
		background: var(--accent-soft);
	}
	.save {
		margin-left: var(--space-4);
	}
	.sess :global(svg),
	.save > :global(svg) {
		flex: none;
		width: 14px;
		height: 14px;
		color: var(--text-muted);
	}
	.t {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		line-height: 1.35;
	}
	.t b {
		display: flex;
		gap: 6px;
		font-weight: 600;
	}
	.t em {
		font-style: normal;
		font-size: var(--text-xs);
		color: var(--accent-text);
	}
	.t small {
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.preset-edit :global(.sliders) {
		grid-template-columns: minmax(0, 1fr);
	}
	.preset-edit :global(.row:not(.stacked)) {
		flex-wrap: wrap;
		gap: var(--space-2);
	}
</style>
