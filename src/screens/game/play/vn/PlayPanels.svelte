<script lang="ts">
	import { ArrowRight, X } from "@lucide/svelte";
	import type { GameCommand } from "$/lib/type/game";
	import { runtimeStore } from "$/lib/game/stores/runtimeStore.svelte";

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


</script>

{#if showMap}
	<aside class="map-panel" aria-labelledby="map-panel-heading">
		<div class="panel-heading">
			<div><strong id="map-panel-heading">{runtimeStore.currentMap?.name ?? "지도"}</strong><small>{runtimeStore.currentLocation?.name ?? "현재 위치 없음"}</small></div>
			<button class="quiet-button" onclick={onCloseMap} aria-label="지도 닫기"><X size={17} /></button>
		</div>
		<div class="map-locations">
			{#each runtimeStore.mapZones as zone (zone.id)}
				<section class="map-zone">
					<h3>{zone.name}</h3>
					{#each zone.locations as location (location.id)}
						{@const isCurrent = location.id === runtimeStore?.currentLocationId}
						{@const destination = runtimeStore.destinations.find((route) => route.toId === location.id)}
						{@const isLocked = Boolean(destination?.requirement && !destination.requirement.available)}
						<button
							type="button"
							class="map-location"
							class:current={isCurrent}
							class:unavailable={!isCurrent && (!destination || isLocked || !canMove)}
							disabled={isLoading || !canMove || isCurrent || !destination || isLocked}
							title={isLocked ? `(${destination?.location?.name})` : undefined}
							onclick={() => destination && onDispatch({ type: "moveTo", locationId: location.id })}
						>
							<span class="map-marker"></span><span>{location.name}</span>
							{#if isCurrent}
								<small>현재 위치</small>
							{:else if !canMove}
								<small>씬 진행 중</small>
							{:else if isLocked}
								<small>({destination?.location?.name})</small>
							{:else if destination}
								<small>이동 · {Math.ceil(destination.travelSeconds / 60)}분</small>
							{:else}
								<small>이동 경로 없음</small>
							{/if}
						</button>
					{/each}
				</section>
			{:else}<p class="muted">표시할 장소가 없습니다.</p>{/each}
		</div>
	</aside>
{/if}

{#if showInventory}
	<aside class="inventory-panel" aria-labelledby="inventory-panel-heading">
		<div class="panel-heading">
			<div><strong id="inventory-panel-heading">보유 아이템</strong><small>{runtimeStore.inventoryEntries.length}종류</small></div>
			<button class="quiet-button" onclick={onCloseInventory} aria-label="인벤토리 닫기"><X size={17} /></button>
		</div>
		{#if runtimeStore.inventoryEntries.length}
			<div class="inventory-list">
				{#each runtimeStore.inventoryEntries as entry (entry.itemId)}
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
		{#if runtimeStore.nearbyCharacters.length}
			<div class="action-group"><span class="action-label">인물과 대화</span>
				{#each runtimeStore.nearbyCharacters as character (character.id)}<button class="destination" disabled={isLoading} onclick={() => onDispatch({ type: "talk", gameCharacterId: character.id })}><span><strong>{character.name}</strong><small>대화 시작</small></span><ArrowRight size={17} /></button>{/each}
			</div>
		{/if}
		{#if runtimeStore.localInteractables.length}
			<div class="action-group"><span class="action-label">주변 살펴보기</span>
				{#each runtimeStore.localInteractables as item (item.id)}<button class="destination" disabled={isLoading} onclick={() => onDispatch({ type: "interact", entityId: item.id })}><span><strong>{item.name}</strong><small>조사</small></span><ArrowRight size={17} /></button>{/each}
			</div>
		{/if}
		{#if canMove && runtimeStore.destinations.length}
			<div class="action-group"><span class="action-label">이동 가능</span>
				{#each runtimeStore.destinations as destination (destination.toId)}
					<button class="destination" class:locked={destination.requirement && !destination.requirement.available} disabled={isLoading || Boolean(destination.requirement && !destination.requirement.available)} onclick={() => onDispatch({ type: "moveTo", locationId: destination.toId })}>
						<span><strong>{destination.location?.name}</strong><small>{Math.ceil(destination.travelSeconds / 60)}분 이동</small>{#if destination.requirement && !destination.requirement.available}<small class="requirement-label">({destination.requirement.toId})</small>{/if}</span>
						<ArrowRight size={17} />
					</button>
				{/each}
			</div>
		{/if}
		{#if runtimeStore.currentChoices.length}
			<div class="action-group"><span class="action-label">현재 선택지</span>
				{#each runtimeStore.currentChoices as choice (choice.id)}<button class="destination" disabled={isLoading} onclick={() => onDispatch({ type: "choose", choiceId: choice.id })}><span><strong>{choice.text}</strong><small>선택</small></span><ArrowRight size={17} /></button>{/each}
			</div>
		{/if}
		{#if !runtimeStore.nearbyCharacters.length && !runtimeStore.localInteractables.length && (!canMove || !runtimeStore.destinations.length) && !runtimeStore.currentChoices.length}<p class="muted">지금 실행할 수 있는 명령이 없습니다.</p>{/if}
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
	.map-location { display: flex; align-items: center; gap: 9px; width: 100%; min-height: 35px; padding: 7px 9px; border: 0; border-radius: var(--radius-md); color: var(--game-text-main); background: transparent; font: inherit; font-size: 13px; text-align: left; cursor: pointer; }
	.map-location:hover:not(:disabled) { background: var(--game-surface-hover); }
	.map-location.unavailable { opacity: .58; cursor: not-allowed; }
	.map-location:disabled { cursor: default; }
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
