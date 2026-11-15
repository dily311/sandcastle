<script lang="ts">
	// 게임 플레이 화면의 오른쪽 패널
	// - 공통 탭(대화 설정 · 컨텍스트·프리셋 · 분기)은 GameSessionTabs
	// - 게임 탭: 상태(지금 장면·주변·변수) / 관계 / 소지품
	// 지도·이동·대사 기록은 무대 위 시트가 맡고, 패널은 늘 펼쳐 두고 볼 정보만
	import { Clock3, CloudSun, Film, MapPin, User } from "@lucide/svelte";
	import { panel } from "$app/panel.svelte";
	import { gameStore } from "../runtime/gameStore.svelte";
	import { runtimeStore } from "../runtime/runtimeStore.svelte";
	import type { RelationshipDetail } from "../types";
	import { formatGameTime } from "./vn/playUtils";
	import GameSessionTabs from "./GameSessionTabs.svelte";

	let { gameId, sessionId }: { gameId: string; sessionId: string } = $props();
	const shared = new Set(["settings", "context", "branches"]);

	const view = $derived(gameStore.view);
	const charName = (id: string) => (id === "player" ? "나" : (view?.characters.find((c) => c.id === id)?.name ?? id));

	// ---- 변수: 지금 세워진 값 ----
	const vars = $derived(
		Object.entries((runtimeStore.flags ?? {}) as Record<string, unknown>).map(([key, v]) => ({
			key,
			desc: null as string | null,
			value: typeof v === "object" ? JSON.stringify(v) : String(v ?? ""),
		})),
	);

	// ---- 관계: "from>to" → 나와 관련된 것 먼저 ----
	const AXES = [
		["affection", "호감"],
		["trust", "신뢰"],
		["tension", "긴장"],
	] as const;
	const relations = $derived(
		Object.entries((runtimeStore.relationships ?? {}) as Record<string, RelationshipDetail>)
			.map(([k, r]) => {
				const [from, to] = k.split(">");
				return { key: k, from, to, r, mine: from === "player" || to === "player" };
			})
			.sort((a, b) => Number(b.mine) - Number(a.mine) || a.key.localeCompare(b.key)),
	);
	const pct = (n: number | undefined) => `${Math.max(0, Math.min(100, Math.abs(n ?? 0)))}%`;
</script>

{#if shared.has(panel.tab)}
	<GameSessionTabs {gameId} {sessionId} />
{:else if panel.tab === "relations"}
	<div class="panel-body">
		{#each relations as rel (rel.key)}
			<section class="rel">
				<h4>{charName(rel.from)} → {charName(rel.to)}</h4>
				{#each AXES as [axis, label] (axis)}
					{#if rel.r[axis] !== undefined}
						<div class="axis">
							<span>{label}</span>
							<span class="bar {axis}" class:neg={(rel.r[axis] ?? 0) < 0}><i style:width={pct(rel.r[axis])}></i></span>
							<b>{rel.r[axis]}</b>
						</div>
					{/if}
				{/each}
				{#if rel.r.tags?.length}<div class="tags">{#each rel.r.tags as t (t)}<span>{t}</span>{/each}</div>{/if}
			</section>
		{:else}
			<p class="panel-none">아직 관계 기록 없음</p>
		{/each}
	</div>
{:else if panel.tab === "items"}
	<div class="panel-body">
		{#each runtimeStore.inventoryEntries as entry (entry.itemId)}
			<div class="item">
				<div class="t">
					<b>{entry.item?.name ?? "알 수 없는 아이템"}</b>
					{#if entry.item?.description}<small>{entry.item.description}</small>{/if}
				</div>
				<span class="qty">× {entry.quantity}</span>
			</div>
		{:else}
			<p class="panel-none">가진 것 없음</p>
		{/each}
	</div>
{:else}
	<div class="panel-body">
		<section>
			<h4>지금</h4>
			<ul class="facts">
				<li><MapPin /><span>{runtimeStore.currentLocation?.name ?? "위치 없음"}</span>{#if runtimeStore.currentMap}<small>{runtimeStore.currentMap.name}</small>{/if}</li>
				<li><Clock3 /><span>{formatGameTime(runtimeStore.gameTime)}</span></li>
				{#if runtimeStore.currentWeather}<li><CloudSun /><span>{runtimeStore.currentWeather.name}</span></li>{/if}
				<li><Film /><span>{runtimeStore.currentScene?.name ?? "자유 행동"}</span>{#if runtimeStore.currentScene}<small>씬 진행 중</small>{/if}</li>
			</ul>
		</section>
		<section>
			<h4>여기 있는 사람 <span class="faint">{runtimeStore.nearbyCharacters.length}</span></h4>
			{#each runtimeStore.nearbyCharacters as c (c.id)}
				<div class="person"><User /><span>{c.name}</span>{#if c.id === runtimeStore.activeNpcId}<small>대화 중</small>{/if}</div>
			{:else}
				<p class="empty">아무도 없음</p>
			{/each}
		</section>
		{#if runtimeStore.localInteractables.length}
			<section>
				<h4>살펴볼 것</h4>
				<div class="tags">{#each runtimeStore.localInteractables as it (it.id)}<span>{it.name}</span>{/each}</div>
			</section>
		{/if}
		{#if vars.length}
			<section>
				<h4>변수</h4>
				<dl class="vars">
					{#each vars as v (v.key)}
						<dt title={v.desc ?? ""}>{v.key}</dt>
						<dd>{v.value}</dd>
					{/each}
				</dl>
			</section>
		{/if}
	</div>
{/if}

<style>
	.facts {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.facts li,
	.person {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;
		font-size: var(--text-sm);
	}
	.facts :global(svg),
	.person :global(svg) {
		width: 15px;
		height: 15px;
		flex: none;
		color: var(--text-muted);
	}
	.facts small,
	.person small {
		margin-left: auto;
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.empty {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}
	.tags span {
		padding: 2px 8px;
		border-radius: var(--radius-full);
		background: var(--surface-2, var(--surface));
		font-size: var(--text-xs);
	}
	.vars {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: 4px var(--space-3);
		margin: 0;
		font-size: var(--text-sm);
	}
	.vars dt {
		font-family: var(--font-mono);
		font-size: 12px;
		color: var(--text-muted);
	}
	.vars dd {
		margin: 0;
		text-align: right;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
	}
	.rel {
		gap: var(--space-2) !important;
	}
	.axis {
		display: grid;
		grid-template-columns: 32px 1fr 36px;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.axis b {
		text-align: right;
		color: var(--text);
		font-variant-numeric: tabular-nums;
	}
	.bar {
		height: 6px;
		border-radius: 3px;
		background: var(--bg-sunken);
		overflow: hidden;
	}
	.bar i {
		display: block;
		height: 100%;
		border-radius: 3px;
		background: var(--accent);
	}
	.bar.tension i {
		background: var(--warning, #d9a441);
	}
	.bar.neg i {
		background: var(--danger, #d9534f);
	}
	.item {
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
		padding-bottom: var(--space-3);
		border-bottom: 1px solid var(--border);
	}
	.item .t {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		font-size: var(--text-sm);
	}
	.item small {
		color: var(--text-muted);
		font-size: var(--text-xs);
		line-height: 1.5;
	}
	.qty {
		font-size: var(--text-sm);
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}
</style>
