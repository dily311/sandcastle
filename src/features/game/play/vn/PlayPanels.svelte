<script lang="ts">
	import { ChevronRight, Lock, MapPin, MessageCircle, Search, Sparkles, X } from "@lucide/svelte";
	import type { GameCommand } from "$features/game/types";
	import { runtimeStore } from "$features/game/runtime/runtimeStore.svelte";

	// 오른쪽에서 열리는 패널: 지도 / 소지품 / 이동·행동
	type Props = {
		showMap: boolean;
		showActions: boolean;
		showInventory: boolean;
		isLoading: boolean;
		canMove: boolean;
		onCloseMap: () => void;
		onCloseActions: () => void;
		onCloseInventory: () => void;
		onDispatch: (command: GameCommand) => void;
	};
	let { showMap, showActions, showInventory, isLoading, canMove, onCloseMap, onCloseActions, onCloseInventory, onDispatch }: Props = $props();

	const minutes = (sec: number) => `${Math.max(1, Math.ceil(sec / 60))}분`;
	const nothingToDo = $derived(
		!runtimeStore.nearbyCharacters.length && !runtimeStore.localInteractables.length && (!canMove || !runtimeStore.destinations.length) && !runtimeStore.currentChoices.length,
	);
</script>

{#if showMap}
	<aside class="g-sheet" aria-labelledby="map-heading">
		<div class="g-sheet-head">
			<div><strong id="map-heading">{runtimeStore.currentMap?.name ?? "지도"}</strong><small>지금 {runtimeStore.currentLocation?.name ?? "위치 없음"}</small></div>
			<button class="g-icon" onclick={onCloseMap} aria-label="지도 닫기 (Esc)"><X /></button>
		</div>
		<div class="g-sheet-body">
			{#if !canMove}<p class="note">씬이 진행 중이라 지금은 이동할 수 없음</p>{/if}
			{#each runtimeStore.mapZones as zone (zone.id)}
				<section class="group">
					<h3>{zone.name}</h3>
					{#each zone.locations as location (location.id)}
						{@const isCurrent = location.id === runtimeStore?.currentLocationId}
						{@const destination = runtimeStore.destinations.find((route) => route.toId === location.id)}
						{@const isLocked = Boolean(destination?.requirement && !destination.requirement.available)}
						<button
							type="button"
							class="row"
							class:current={isCurrent}
							disabled={isLoading || !canMove || isCurrent || !destination || isLocked}
							onclick={() => destination && onDispatch({ type: "moveTo", locationId: location.id })}
						>
							<span class="ic">{#if isCurrent}<MapPin />{:else if isLocked}<Lock />{:else}<span class="dot"></span>{/if}</span>
							<span class="t"><b>{location.name}</b>
								<small>
									{#if isCurrent}지금 여기{:else if isLocked}조건이 필요함{:else if destination}{minutes(destination.travelSeconds)} 걸림{:else}가는 길 없음{/if}
								</small>
							</span>
							{#if destination && !isCurrent && !isLocked && canMove}<ChevronRight />{/if}
						</button>
					{/each}
				</section>
			{:else}<p class="note">표시할 장소 없음</p>{/each}
		</div>
	</aside>
{/if}

{#if showInventory}
	<aside class="g-sheet" aria-labelledby="inv-heading">
		<div class="g-sheet-head">
			<div><strong id="inv-heading">소지품</strong><small>{runtimeStore.inventoryEntries.length}종류</small></div>
			<button class="g-icon" onclick={onCloseInventory} aria-label="소지품 닫기 (Esc)"><X /></button>
		</div>
		<div class="g-sheet-body">
			{#each runtimeStore.inventoryEntries as entry (entry.itemId)}
				<article class="item">
					<span class="item-ic" aria-hidden="true">{entry.item?.icon ? "✦" : "◇"}</span>
					<div class="t">
						<b>{entry.item?.name ?? "알 수 없는 아이템"}</b>
						{#if entry.item?.description}<p>{entry.item.description}</p>{/if}
						{#if entry.item?.isKey || entry.item?.consumable}
							<div class="tags">{#if entry.item?.isKey}<span>중요</span>{/if}{#if entry.item?.consumable}<span>소모품</span>{/if}</div>
						{/if}
					</div>
					<span class="qty">× {entry.quantity}</span>
				</article>
			{:else}
				<p class="note">아직 가진 아이템 없음</p>
			{/each}
		</div>
	</aside>
{/if}

{#if showActions}
	<aside class="g-sheet" aria-labelledby="act-heading">
		<div class="g-sheet-head">
			<div><strong id="act-heading">이동 / 행동</strong><small>{runtimeStore.currentLocation?.name ?? ""}에서 할 수 있는 것</small></div>
			<button class="g-icon" onclick={onCloseActions} aria-label="닫기 (Esc)"><X /></button>
		</div>
		<div class="g-sheet-body">
			{#if runtimeStore.currentChoices.length}
				<section class="group">
					<h3>선택지</h3>
					{#each runtimeStore.currentChoices as choice (choice.id)}
						<button class="row" disabled={isLoading} onclick={() => onDispatch({ type: "choose", choiceId: choice.id })}>
							<span class="ic"><Sparkles /></span><span class="t"><b>{choice.text}</b></span><ChevronRight />
						</button>
					{/each}
				</section>
			{/if}
			{#if runtimeStore.nearbyCharacters.length}
				<section class="group">
					<h3>대화</h3>
					{#each runtimeStore.nearbyCharacters as character (character.id)}
						<button class="row" disabled={isLoading} onclick={() => onDispatch({ type: "talk", gameCharacterId: character.id })}>
							<span class="ic"><MessageCircle /></span><span class="t"><b>{character.name}</b><small>말 걸기</small></span><ChevronRight />
						</button>
					{/each}
				</section>
			{/if}
			{#if runtimeStore.localInteractables.length}
				<section class="group">
					<h3>살펴보기</h3>
					{#each runtimeStore.localInteractables as item (item.id)}
						<button class="row" disabled={isLoading} onclick={() => onDispatch({ type: "interact", entityId: item.id })}>
							<span class="ic"><Search /></span><span class="t"><b>{item.name}</b><small>조사</small></span><ChevronRight />
						</button>
					{/each}
				</section>
			{/if}
			{#if canMove && runtimeStore.destinations.length}
				<section class="group">
					<h3>이동</h3>
					{#each runtimeStore.destinations as destination (destination.toId)}
						{@const locked = Boolean(destination.requirement && !destination.requirement.available)}
						<button class="row" disabled={isLoading || locked} onclick={() => onDispatch({ type: "moveTo", locationId: destination.toId })}>
							<span class="ic">{#if locked}<Lock />{:else}<MapPin />{/if}</span>
							<span class="t"><b>{destination.location?.name}</b><small>{locked ? "조건이 필요함" : `${minutes(destination.travelSeconds)} 걸림`}</small></span>
							{#if !locked}<ChevronRight />{/if}
						</button>
					{/each}
				</section>
			{/if}
			{#if nothingToDo}<p class="note">지금 할 수 있는 것 없음</p>{/if}
		</div>
	</aside>
{/if}

<style>
	.group + .group {
		margin-top: 14px;
	}
	h3 {
		margin: 2px 6px 6px;
		color: var(--game-text-dim);
		font-size: 12px;
		font-weight: 600;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 9px 10px;
		border: 1px solid transparent;
		border-radius: 12px;
		background: transparent;
		color: var(--game-text-main);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.row:hover:not(:disabled) {
		border-color: var(--game-accent-border);
		background: var(--game-accent-soft);
	}
	.row:disabled {
		cursor: default;
		opacity: 0.55;
	}
	.row.current {
		opacity: 1;
		background: var(--game-accent-soft);
	}
	.row > :global(svg) {
		width: 16px;
		height: 16px;
		color: var(--game-text-dim);
	}
	.ic {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		flex-shrink: 0;
		border-radius: 9px;
		background: rgb(255 255 255 / 0.06);
		color: var(--game-accent);
	}
	.ic :global(svg) {
		width: 15px;
		height: 15px;
	}
	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--game-text-dim);
	}
	.t {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.t b {
		color: var(--game-text-primary);
		font-size: 14px;
	}
	.t small {
		color: var(--game-text-muted);
		font-size: 12px;
	}
	.t p {
		margin: 2px 0 0;
		color: var(--game-text-muted);
		font-size: 13px;
		line-height: 1.5;
	}
	.item {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		padding: 10px;
		border-radius: 12px;
	}
	.item + .item {
		border-top: 1px solid var(--game-border-subtle);
	}
	.item-ic {
		display: grid;
		place-items: center;
		width: 34px;
		height: 34px;
		flex-shrink: 0;
		border-radius: 10px;
		background: var(--game-accent-soft);
		color: var(--game-accent);
	}
	.qty {
		color: var(--game-text-secondary);
		font-size: 13px;
		font-variant-numeric: tabular-nums;
	}
	.tags {
		display: flex;
		gap: 4px;
		margin-top: 6px;
	}
	.tags span {
		padding: 1px 8px;
		border-radius: 999px;
		background: rgb(255 255 255 / 0.07);
		color: var(--game-text-secondary);
		font-size: 11px;
	}
	.note {
		margin: 8px 6px;
		color: var(--game-text-muted);
		font-size: 13px;
	}
</style>
