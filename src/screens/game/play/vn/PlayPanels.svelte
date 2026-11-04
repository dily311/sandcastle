<script lang="ts">
	import { ArrowRight, X } from "@lucide/svelte";
	import { gameStore } from "$/lib/game/gameStore.svelte";
	import type { GameCommand } from "$/lib/type/game";
	import { getConnectionRequirement } from "./playUtils";

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

	const choices = $derived(gameStore.choices ?? []);
	const destinations = $derived(
		(gameStore.locations ?? []).map((destination) => ({
			...destination,
			requirement: getConnectionRequirement(destination.condition, gameStore.state, gameStore.view),
		})),
	);
	const inventoryEntries = $derived(
		Object.entries((gameStore.state?.inventory ?? {}) as Record<string, number>)
			.filter(([, quantity]) => typeof quantity === "number" && quantity > 0)
			.map(([itemId, quantity]) => ({ itemId, quantity, item: gameStore.view?.items.find((item) => item.id === itemId) })),
	);
	const mapZones = $derived(
		(gameStore.view?.zones ?? [])
			.filter((zone) => zone.mapId === gameStore.state?.mapId)
			.map((zone) => ({ ...zone, locations: (gameStore.view?.locations ?? []).filter((location) => location.zoneId === zone.id) })),
	);
	const nearbyCharacters = $derived(
		(gameStore.view?.characters ?? []).filter((character) => gameStore.state?.presentNpcs.includes(character.id)),
	);
	const localInteractables = $derived(
		(gameStore.view?.interactables ?? []).filter((item) => item.locationId === gameStore.state?.locationId),
	);
</script>

{#if showMap}
	<aside class="map-panel" aria-labelledby="map-panel-heading">
		<div class="panel-heading">
			<div><strong id="map-panel-heading">{gameStore.currentMap?.name ?? "지도"}</strong><small>{gameStore.currentLocation?.name ?? "현재 위치 없음"}</small></div>
			<button class="quiet-button" onclick={onCloseMap} aria-label="지도 닫기"><X size={17} /></button>
		</div>
		<div class="map-locations">
			{#each mapZones as zone (zone.id)}
				<section class="map-zone">
					<h3>{zone.name}</h3>
					{#each zone.locations as location (location.id)}
						<div class="map-location" class:current={location.id === gameStore.state?.locationId}>
							<span class="map-marker"></span><span>{location.name}</span>
							{#if location.id === gameStore.state?.locationId}<small>현재 위치</small>{:else if destinations.some((destination) => destination.toId === location.id)}<small>이동 경로 있음</small>{/if}
						</div>
					{/each}
				</section>
			{:else}<p class="muted">표시할 장소가 없습니다.</p>{/each}
		</div>
	</aside>
{/if}

{#if showInventory}
	<aside class="inventory-panel" aria-labelledby="inventory-panel-heading">
		<div class="panel-heading">
			<div><strong id="inventory-panel-heading">보유 아이템</strong><small>{inventoryEntries.length}종류</small></div>
			<button class="quiet-button" onclick={onCloseInventory} aria-label="인벤토리 닫기"><X size={17} /></button>
		</div>
		{#if inventoryEntries.length}
			<div class="inventory-list">
				{#each inventoryEntries as entry (entry.itemId)}
					<article class="inventory-item">
						<div class="item-icon" aria-hidden="true">{entry.item?.icon ? "✦" : "◇"}</div>
						<div class="item-copy">
							<strong>{entry.item?.name ?? "알 수 없는 아이템"}</strong>
							{#if entry.item?.description}<p>{entry.item.description}</p>{/if}
							<div class="item-tags">{#if entry.item?.isKey}<span>중요</span>{/if}{#if entry.item?.consumable}<span>소모품</span>{/if}</div>
						</div>
						<span class="item-quantity">× {entry.quantity}</span>
					</article>
				{/each}
			</div>
		{:else}<p class="empty-inventory">아직 가지고 있는 아이템이 없습니다.</p>{/if}
	</aside>
{/if}

{#if showActions}
	<aside class="actions-panel" aria-labelledby="actions-panel-heading">
		<div class="panel-heading"><div><strong id="actions-panel-heading">이동 / 행동</strong><small>지금 실행할 수 있는 명령</small></div><button class="quiet-button" onclick={onCloseActions} aria-label="행동 메뉴 닫기"><X size={17} /></button></div>
		{#if nearbyCharacters.length}
			<div class="action-group"><span class="action-label">인물과 대화</span>
				{#each nearbyCharacters as character (character.id)}<button class="destination" disabled={isLoading} onclick={() => onDispatch({ type: "talk", gameCharacterId: character.id })}><span><strong>{character.name}</strong><small>대화 시작</small></span><ArrowRight size={17} /></button>{/each}
			</div>
		{/if}
		{#if localInteractables.length}
			<div class="action-group"><span class="action-label">주변 살펴보기</span>
				{#each localInteractables as item (item.id)}<button class="destination" disabled={isLoading} onclick={() => onDispatch({ type: "interact", entityId: item.id })}><span><strong>{item.name}</strong><small>조사</small></span><ArrowRight size={17} /></button>{/each}
			</div>
		{/if}
		{#if canMove && destinations.length}
			<div class="action-group"><span class="action-label">이동 가능</span>
				{#each destinations as destination (destination.toId)}
					<button class="destination" class:locked={destination.requirement && !destination.requirement.isMet} disabled={isLoading || Boolean(destination.requirement && !destination.requirement.isMet)} onclick={() => onDispatch({ type: "moveTo", locationId: destination.toId })}>
						<span><strong>{destination.location?.name}</strong><small>{Math.ceil(destination.travelSeconds / 60)}분 이동</small>{#if destination.requirement && !destination.requirement.isMet}<small class="requirement-label">({destination.requirement.label})</small>{/if}</span>
						<ArrowRight size={17} />
					</button>
				{/each}
			</div>
		{/if}
		{#if choices.length}
			<div class="action-group"><span class="action-label">현재 선택지</span>
				{#each choices as choice (choice.id)}<button class="destination" disabled={isLoading} onclick={() => onDispatch({ type: "choose", choiceId: choice.id })}><span><strong>{choice.text}</strong><small>선택</small></span><ArrowRight size={17} /></button>{/each}
			</div>
		{/if}
		{#if !nearbyCharacters.length && !localInteractables.length && (!canMove || !destinations.length) && !choices.length}<p class="muted">지금 실행할 수 있는 명령이 없습니다.</p>{/if}
	</aside>
{/if}

<style>
	.map-panel, .actions-panel, .inventory-panel { position: absolute; z-index: 120; right: 18px; width: min(320px, calc(100% - 36px)); padding: 14px; border: 1px solid var(--game-border); border-radius: 14px; background: var(--game-panel-bg); box-shadow: var(--game-shadow-panel); backdrop-filter: blur(14px); }
	.map-panel, .inventory-panel { top: 16px; }
	.actions-panel { bottom: 16px; max-height: calc(100% - 32px); overflow-y: auto; }
	.panel-heading { display: flex; align-items: center; justify-content: space-between; padding-left: 7px; margin-bottom: 8px; font-size: 14px; }
	.panel-heading > div { display: flex; flex-direction: column; gap: 5px; }
	.panel-heading small { color: var(--game-text-dim); font-size: 12px; }
	.quiet-button { display: inline-grid; place-items: center; width: 34px; height: 34px; border: 0; border-radius: 50%; color: var(--game-text-main); background: transparent; cursor: pointer; }
	.quiet-button:hover { background: var(--game-surface-hover); }
	.map-locations { display: grid; gap: 13px; max-height: min(60vh, 520px); overflow-y: auto; padding: 4px; }
	.map-zone h3 { margin: 0 0 7px; color: var(--game-text-muted); font-size: 11px; text-transform: uppercase; letter-spacing: .08em; }
	.map-location { display: flex; align-items: center; gap: 9px; min-height: 35px; padding: 7px 9px; border-radius: var(--radius-md); color: var(--game-text-main); font-size: 13px; }
	.map-location.current { background: var(--game-accent-soft); color: var(--game-accent-strong); }
	.map-location small { margin-left: auto; color: var(--game-text-muted); font-size: 10px; }
	.map-marker { width: 8px; height: 8px; flex: 0 0 8px; border: 1px solid var(--game-accent); border-radius: 50%; }
	.map-location.current .map-marker { border-color: var(--game-accent); background: var(--game-accent); }
	.action-group { display: grid; gap: 2px; margin-top: 10px; }
	.action-label { padding: 5px 9px; color: var(--game-text-muted); font-size: 11px; }
	.destination { display: flex; align-items: center; justify-content: space-between; width: 100%; padding: 11px 9px; border: 0; border-radius: 9px; color: var(--game-text-main); background: transparent; text-align: left; cursor: pointer; }
	.destination:hover:not(:disabled) { background: var(--game-surface-hover); }
	.destination:disabled { opacity: .55; cursor: wait; }
	.destination.locked { opacity: .62; cursor: not-allowed; }
	.destination .requirement-label { color: var(--game-accent-strong); opacity: .8; }
	.destination span { display: flex; flex-direction: column; gap: 3px; }
	.destination small { color: var(--game-text-dim); }
	.inventory-list { display: grid; gap: 7px; max-height: min(62vh, 540px); overflow-y: auto; padding: 3px; }
	.inventory-item { display: flex; align-items: flex-start; gap: 10px; padding: 10px 8px; border: 1px solid var(--game-border-subtle); border-radius: var(--radius-lg); background: var(--game-border-subtle); }
	.item-icon { display: grid; place-items: center; width: 34px; height: 34px; flex: 0 0 34px; border-radius: var(--radius-md); color: var(--game-accent); background: var(--game-accent-soft); font-size: 18px; }
	.item-copy { flex: 1; min-width: 0; }
	.item-copy > strong { display: block; color: var(--game-text-primary); font-size: 13px; font-weight: 650; }
	.item-copy p { margin-top: 5px; color: var(--game-text-muted); font-size: 11px; line-height: 1.5; }
	.item-tags { display: flex; gap: 5px; margin-top: 6px; }
	.item-tags span { padding: 3px 6px; border-radius: 99px; color: var(--game-accent-strong); background: var(--game-accent-soft); font-size: 10px; }
	.item-quantity { flex: 0 0 auto; color: var(--game-accent-strong); font-size: 12px; font-weight: 650; }
	.empty-inventory, .muted { padding: 18px 8px; color: var(--game-text-muted); font-size: 13px; line-height: 1.5; text-align: center; }
</style>
