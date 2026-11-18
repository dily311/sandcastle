<script lang="ts">
	// 게임 패널: 이동·행동 / 지도 (예전에는 무대 위 시트였음)
	import { ChevronRight, Lock, MapPin, MessageCircle, Search, Sparkles } from "@lucide/svelte";
	import { panel } from "$app/panel.svelte";
	import type { GameCommand } from "../types";
	import { playBridge } from "../runtime/playBridge.svelte";
	import StoryPlanPanel from "./StoryPlanPanel.svelte";
	import { runtimeStore } from "../runtime/runtimeStore.svelte";

	const canMove = $derived(runtimeStore.canWait);
	const busy = $derived(playBridge.loading || !playBridge.dispatch);
	const send = (command: GameCommand) => playBridge.dispatch?.(command);
	const minutes = (sec: number) => `${Math.max(1, Math.ceil(sec / 60))}분`;
	const nothingToDo = $derived(
		!runtimeStore.nearbyCharacters.length && !runtimeStore.localInteractables.length && (!canMove || !runtimeStore.destinations.length) && !runtimeStore.currentChoices.length,
	);
</script>

{#if panel.tab === "map"}
	<div class="panel-body">
		<section>
			<h4>{runtimeStore.currentMap?.name ?? "지도"} <span class="faint">지금 {runtimeStore.currentLocation?.name ?? "위치 없음"}</span></h4>
			{#if !canMove}<p class="note">씬이 진행 중이라 지금은 이동할 수 없음</p>{/if}
		</section>
		{#each runtimeStore.mapZones as zone (zone.id)}
			<section>
				<h4>{zone.name}</h4>
				<div class="list">
					{#each zone.locations as location (location.id)}
						{@const here = location.id === runtimeStore.currentLocationId}
						{@const route = runtimeStore.destinations.find((r) => r.toId === location.id)}
						{@const block = route ? runtimeStore.moveBlock(location.id) : null}
						{@const locked = Boolean(block)}
						<button class="row" class:here disabled={busy || !canMove || here || !route || locked} onclick={() => route && send({ type: "moveTo", locationId: location.id })}>
							<span class="ic">{#if here}<MapPin />{:else if locked}<Lock />{:else}<span class="dot"></span>{/if}</span>
							<span class="t">
								<b>{location.name}</b>
								<small>{#if here}지금 여기{:else if block}{block}{:else if route}{minutes(route.travelSeconds)} 걸림{:else}가는 길 없음{/if}</small>
							</span>
							{#if route && !here && !locked && canMove}<ChevronRight />{/if}
						</button>
					{/each}
				</div>
			</section>
		{:else}
			<p class="note">표시할 장소 없음</p>
		{/each}
	</div>
{:else}
	<div class="panel-body">
		<section>
			<h4>이동·행동 <span class="faint">{runtimeStore.currentLocation?.name ?? ""}</span></h4>
			{#if !playBridge.dispatch}<p class="note">플레이 화면이 열려 있어야 함</p>{/if}
		</section>
		{#if runtimeStore.currentChoices.length}
			<section>
				<h4>선택지</h4>
				<div class="list">
					{#each runtimeStore.currentChoices as choice (choice.id)}
						<button class="row" disabled={busy} onclick={() => send({ type: "choose", choiceId: choice.id })}>
							<span class="ic"><Sparkles /></span><span class="t"><b>{choice.text}</b></span><ChevronRight />
						</button>
					{/each}
				</div>
			</section>
		{/if}
		{#if runtimeStore.nearbyCharacters.length}
			<section>
				<h4>대화</h4>
				<div class="list">
					{#each runtimeStore.nearbyCharacters as character (character.id)}
						<!-- 씬 동안은 1:1 대화가 막힌다 (엔진이 sceneActive로 거절). 대사창의 씬 입력을 쓴다 -->
						<button class="row" disabled={busy || Boolean(runtimeStore.currentSceneId)} onclick={() => send({ type: "talk", gameCharacterId: character.id })}>
							<span class="ic"><MessageCircle /></span><span class="t"><b>{character.name}</b><small>{runtimeStore.currentSceneId ? "장면이 끝나면 말 걸 수 있음" : "말 걸기"}</small></span><ChevronRight />
						</button>
					{/each}
				</div>
			</section>
		{/if}
		{#if runtimeStore.localInteractables.length}
			<section>
				<h4>살펴보기</h4>
				<div class="list">
					{#each runtimeStore.localInteractables as item (item.id)}
						<button class="row" disabled={busy} onclick={() => send({ type: "interact", entityId: item.id })}>
							<span class="ic"><Search /></span><span class="t"><b>{item.name}</b><small>조사</small></span><ChevronRight />
						</button>
					{/each}
				</div>
			</section>
		{/if}
		{#if canMove && runtimeStore.destinations.length}
			<section>
				<h4>이동</h4>
				<div class="list">
					{#each runtimeStore.destinations as d (d.toId)}
						{@const block = runtimeStore.moveBlock(d.toId)}
						{@const locked = Boolean(block)}
						<button class="row" disabled={busy || locked} onclick={() => send({ type: "moveTo", locationId: d.toId })}>
							<span class="ic">{#if locked}<Lock />{:else}<MapPin />{/if}</span>
							<span class="t"><b>{d.location?.name}</b><small>{block ?? `${minutes(d.travelSeconds)} 걸림`}</small></span>
							{#if !locked}<ChevronRight />{/if}
						</button>
					{/each}
				</div>
			</section>
		{/if}
		{#if nothingToDo}<p class="note">지금 할 수 있는 것 없음</p>{/if}
		<StoryPlanPanel />
	</div>
{/if}

<style>
	.list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		padding: 8px var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.row:hover:not(:disabled) {
		background: var(--surface-hover);
	}
	.row:disabled {
		cursor: default;
		color: var(--text-muted);
	}
	.row.here {
		background: var(--accent-soft);
		color: var(--text);
	}
	.row > :global(svg) {
		flex: none;
		width: 15px;
		height: 15px;
		color: var(--text-faint);
	}
	.ic {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		flex: none;
		border-radius: var(--radius-sm);
		background: var(--surface);
		color: var(--accent-text);
	}
	.ic :global(svg) {
		width: 15px;
		height: 15px;
	}
	.dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-faint);
	}
	.t {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		font-size: var(--text-sm);
		line-height: 1.35;
	}
	.t small {
		color: var(--text-muted);
		font-size: var(--text-xs);
	}
	.note {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
</style>
