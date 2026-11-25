<script lang="ts">
	import { _ } from "svelte-i18n";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	/** 지우기 전에 앱 확인 창으로 물어봄 */
	const ask = (message: string) => confirmAction({ title: $_("common.delete"), message, confirmLabel: $_("common.delete"), danger: true });
	import type {
		GameBackgroundAssetsInput,
		GameInteractablesInput,
		GameEventsInput,
		GameLocationConnectionsInput,
		GameLocationsInput,
		GameMapsInput,
		GameMapsZoneInput,
	} from "$features/game/types";
	import { editorAssetPreviewUrl } from "./assetPreview";

	type Props = {
		maps: GameMapsInput[];
		zones: GameMapsZoneInput[];
		locations: GameLocationsInput[];
		gameId: string;
		backgroundAssets: GameBackgroundAssetsInput[];
		timePeriods: { id: string | null; name: string }[];
		weatherTypes: { id: string | null; name: string }[];
		connections: GameLocationConnectionsInput[];
		interactables: GameInteractablesInput[];
		events: GameEventsInput[];
		assetRoot: string;
	};
	let {
		maps = $bindable([]),
		zones = $bindable([]),
		locations = $bindable([]),
		backgroundAssets = $bindable([]),
		connections = $bindable([]),
		interactables = $bindable([]),
		events,
		gameId,
		timePeriods,
		weatherTypes,
		assetRoot,
	}: Props = $props();

	// 현재 선택된 항목들의 인덱스 또는 ID 관리
	let selectedMapIndex = $state<number | null>(maps.length > 0 ? 0 : null);
	let selectedZoneIndex = $state<number | null>(null);
	let selectedLocationIndex = $state<number | null>(null);
	let hierarchySearch = $state("");
	let filteredMaps = $derived(maps.map((map, index) => ({ map, index })).filter(({ map }) => map.name.toLowerCase().includes(hierarchySearch.trim().toLowerCase())));

	// 1. 현재 선택된 맵
	let activeMap = $derived(selectedMapIndex !== null && maps[selectedMapIndex] ? maps[selectedMapIndex] : null);

	// 2. 현재 선택된 맵에 속한 존(Zone)들 필터링
	let currentZones = $derived(activeMap ? zones.filter((z) => z.mapId === activeMap.id) : []);
	let filteredZones = $derived(currentZones.map((zone, index) => ({ zone, index })).filter(({ zone }) => zone.name.toLowerCase().includes(hierarchySearch.trim().toLowerCase())));
	let activeZone = $derived(
		selectedZoneIndex !== null && currentZones[selectedZoneIndex] ? currentZones[selectedZoneIndex] : null,
	);

	// 3. 현재 선택된 존에 속한 로케이션(Location)들 필터링
	let currentLocations = $derived(activeZone ? locations.filter((l) => l.zoneId === activeZone.id) : []);
	let filteredLocations = $derived(currentLocations.map((location, index) => ({ location, index })).filter(({ location }) => location.name.toLowerCase().includes(hierarchySearch.trim().toLowerCase()) || interactables.some((item) => item.locationId === location.id && item.name.toLowerCase().includes(hierarchySearch.trim().toLowerCase()))));
	let activeLocation = $derived(
		selectedLocationIndex !== null && currentLocations[selectedLocationIndex]
			? currentLocations[selectedLocationIndex]
			: null,
	);
	let activeLocationRoutes = $derived(
		activeLocation?.id
			? connections
					.map((connection, index) => ({ connection, index }))
					.filter(
						({ connection }) =>
							connection.fromId === activeLocation?.id || connection.toId === activeLocation?.id,
					)
			: [],
	);
	let activeLocationInteractables = $derived(
		activeLocation?.id
			? interactables
					.map((item, index) => ({ item, index }))
					.filter(({ item }) => item.locationId === activeLocation?.id)
			: [],
	);

	// --- 생성 및 삭제 핸들러 ---

	// 맵 추가
	function handleAddMap() {
		const newMap: GameMapsInput = {
			id: crypto.randomUUID(),
			gameId: gameId,
			name: $_("gameEditor.newMapName"),
			dangerLevel: 1,
			defaultWeatherPoolId: null,
			settingsJson: null,
		};
		maps.push(newMap);
		selectedMapIndex = maps.length - 1;
		selectedZoneIndex = null;
		selectedLocationIndex = null;
	}

	// 존(Zone) 추가 (현재 선택된 맵에 종속)
	function handleAddZone() {
		if (!activeMap || !activeMap.id) return;
		const newZone: GameMapsZoneInput = {
			id: crypto.randomUUID(),
			mapId: activeMap.id,
			name: $_("gameEditor.newZoneName"),
			tags: [],
			settingsJson: null,
		};
		zones.push(newZone);
		selectedZoneIndex = currentZones.length; // 추가된 위치
		selectedLocationIndex = null;
	}

	// 로케이션(Location) 추가 (현재 선택된 존에 종속)
	function handleAddLocation() {
		if (!activeZone || !activeZone.id) return;
		const newLoc: GameLocationsInput = {
			id: crypto.randomUUID(),
			zoneId: activeZone.id,
			name: $_("gameEditor.newLocationName"),
			description: "",
			settingsJson: null,
		};
		locations.push(newLoc);
		selectedLocationIndex = currentLocations.length;
	}

	// 삭제 핸들러들
	async function handleDeleteMap(index: number) {
		const targetMap = maps[index];
		if (!await ask($_("gameEditor.deleteMapConfirm", { values: { name: targetMap.name } }))) return;

		// 연관된 존과 로케이션도 정리
		if (targetMap.id) {
			const zoneIdsToRemove = zones.filter((z) => z.mapId === targetMap.id).map((z) => z.id);
			const locationIdsToRemove = locations
				.filter((location) => location.zoneId && zoneIdsToRemove.includes(location.zoneId))
				.map((location) => location.id);
			zones = zones.filter((z) => z.mapId !== targetMap.id);
			locations = locations.filter((l) => l.zoneId && !zoneIdsToRemove.includes(l.zoneId));
			backgroundAssets = backgroundAssets.filter(
				(asset) => asset.locationId === null || !locationIdsToRemove.includes(asset.locationId),
			);
			connections = connections.filter(
				(connection) =>
					!locationIdsToRemove.includes(connection.fromId) && !locationIdsToRemove.includes(connection.toId),
			);
			interactables = interactables.filter((item) => !locationIdsToRemove.includes(item.locationId));
		}

		maps.splice(index, 1);
		selectedMapIndex = maps.length > 0 ? 0 : null;
		selectedZoneIndex = null;
		selectedLocationIndex = null;
	}

	async function handleDeleteZone(zoneId: string | null) {
		if (!zoneId || !await ask($_("gameEditor.deleteZoneConfirm"))) return;

		const locationIdsToRemove = locations
			.filter((location) => location.zoneId === zoneId)
			.map((location) => location.id);
		zones = zones.filter((z) => z.id !== zoneId);
		locations = locations.filter((l) => l.zoneId !== zoneId);
		backgroundAssets = backgroundAssets.filter(
			(asset) => asset.locationId === null || !locationIdsToRemove.includes(asset.locationId),
		);
		connections = connections.filter(
			(connection) =>
				!locationIdsToRemove.includes(connection.fromId) && !locationIdsToRemove.includes(connection.toId),
		);
		interactables = interactables.filter((item) => !locationIdsToRemove.includes(item.locationId));
		selectedZoneIndex = null;
		selectedLocationIndex = null;
	}

	async function handleDeleteLocation(locId: string | null) {
		if (!locId || !await ask($_("gameEditor.deleteLocationConfirm"))) return;

		locations = locations.filter((l) => l.id !== locId);
		backgroundAssets = backgroundAssets.filter((asset) => asset.locationId !== locId);
		connections = connections.filter((connection) => connection.fromId !== locId && connection.toId !== locId);
		interactables = interactables.filter((item) => item.locationId !== locId);
		selectedLocationIndex = null;
	}

	function handleAddRoute() {
		const locationId = activeLocation?.id;
		const destination = locations.find(
			(location) =>
				location.id &&
				location.id !== locationId &&
				!connections.some((connection) => connection.fromId === locationId && connection.toId === location.id),
		);
		if (!locationId || !destination?.id) return;
		connections.push({
			fromId: locationId,
			toId: destination.id,
			travelSeconds: 60,
			condition: null,
			settingsJson: null,
		});
	}

	function canAddRoute() {
		const locationId = activeLocation?.id;
		return (
			!!locationId &&
			locations.some(
				(location) =>
					location.id &&
					location.id !== locationId &&
					!connections.some(
						(connection) => connection.fromId === locationId && connection.toId === location.id,
					),
			)
		);
	}

	function handleDeleteRoute(index: number) {
		connections.splice(index, 1);
	}

	function handleAddInteractable() {
		if (!activeLocation?.id) return;
		interactables.push({
			id: crypto.randomUUID(),
			locationId: activeLocation.id,
			name: $_("gameEditor.newInteractableName"),
			eventId: null,
		});
	}

	function handleDeleteInteractable(index: number) {
		interactables.splice(index, 1);
	}

	function handleAddBackground() {
		if (!activeLocation?.id) return;
		backgroundAssets.push({
			id: crypto.randomUUID(),
			gameId,
			key: `background_${Date.now()}`,
			name: $_("gameEditor.bgDefaultName", { values: { name: activeLocation.name } }),
			category: "background",
			kind: "image",
			filePath: "",
			settingsJson: null,
			locationId: activeLocation.id,
			timeId: null,
			weatherId: null,
		});
	}

	function removeBackground(id: string | null) {
		backgroundAssets = backgroundAssets.filter((asset) => asset.id !== id);
	}
</script>

<div class="map-hierarchy-layout">
	<section class="item-list-pane">
		<label class="list-search"><span aria-hidden="true">⌕</span><input aria-label={$_("gameEditor.searchHierarchyAria")} placeholder={$_("gameEditor.searchHierarchyPlaceholder")} bind:value={hierarchySearch} /></label>
		<!-- 1단: 맵(Map) 선택/목록 영역 -->
		<div class="hierarchy-level">
			<div class="pane-header">
				<h4>{$_("gameEditor.mapsHeading")}</h4>
				<button class="add-btn" onclick={handleAddMap}>{$_("gameEditor.addMapBtn")}</button>
			</div>
			<div class="scroll-list">
				{#each filteredMaps as row (row.map.id || row.index)}
					{@const map = row.map}{@const idx = row.index}
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="list-item"
						class:active={selectedMapIndex === idx}
						onclick={() => {
							selectedMapIndex = idx;
							selectedZoneIndex = null;
							selectedLocationIndex = null;
						}}
					>
						<span class="title">{map.name}</span>
						<button class="icon-del" onclick={() => handleDeleteMap(idx)}>×</button>
					</div>
				{:else}
					<div class="empty-text">{$_("gameEditor.noMaps")}</div>
				{/each}
			</div>
		</div>

		<!-- 2단: 존(Zone) 선택/목록 영역 (선택된 맵이 있을 때만 활성) -->
		<div class="hierarchy-level">
			<div class="pane-header">
				<h4>{$_("gameEditor.zonesHeading")}</h4>
				<button class="add-btn" onclick={handleAddZone} disabled={!activeMap}>{$_("gameEditor.addZoneBtn")}</button>
			</div>
			<div class="scroll-list">
				{#if activeMap}
					{#each filteredZones as row (row.zone.id || row.index)}
						{@const zone = row.zone}{@const idx = row.index}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="list-item"
							class:active={selectedZoneIndex === idx}
							onclick={() => {
								selectedZoneIndex = idx;
								selectedLocationIndex = null;
							}}
						>
							<span class="title">{zone.name}</span>
							<button class="icon-del" onclick={() => handleDeleteZone(zone.id)}>×</button>
						</div>
					{:else}
						<div class="empty-text">{$_("gameEditor.noZones")}</div>
					{/each}
				{:else}
					<div class="empty-text">{$_("gameEditor.selectMapFirst")}</div>
				{/if}
			</div>
		</div>

		<!-- 3단: 로케이션(Location) 선택/목록 영역 (선택된 존이 있을 때만 활성) -->
		<div class="hierarchy-level">
			<div class="pane-header">
				<h4>{$_("gameEditor.locationsHeading")}</h4>
				<button class="add-btn" onclick={handleAddLocation} disabled={!activeZone}>{$_("gameEditor.addLocationBtn")}</button>
			</div>
			<div class="scroll-list">
				{#if activeZone}
					{#each filteredLocations as row (row.location.id || row.index)}
						{@const loc = row.location}{@const idx = row.index}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="list-item"
							class:active={selectedLocationIndex === idx}
							onclick={() => (selectedLocationIndex = idx)}
						>
							<span class="location-row-copy"
								><span class="title">{loc.name}</span><span class="location-row-meta"
									>{interactables
										.filter((item) => item.locationId === loc.id)
										.map((item) => item.name)
										.join(" · ") || $_("gameEditor.noInteractablesShort")}</span
								></span
							>
							<button class="icon-del" onclick={() => handleDeleteLocation(loc.id)}>×</button>
						</div>
					{:else}
						<div class="empty-text">{$_("gameEditor.noLocations")}</div>
					{/each}
				{:else}
					<div class="empty-text">{$_("gameEditor.selectZoneFirst")}</div>
				{/if}
			</div>
		</div>
	</section>

	<!-- 선택한 맵/존/장소를 편집하는 넓은 패널 -->
	<div class="item-detail-pane">
		{#if activeLocation}
			<div class="detail-header">
				<h3>{$_("gameEditor.editLocationHeading")} <strong>{activeLocation.name}</strong></h3>
			</div>
			<div class="form-grid">
				<div class="field">
					<label for="location_name">{$_("gameEditor.locationNameLabel")}</label>
					<input id="location_name" type="text" bind:value={activeLocation.name} />
				</div>
				<details class="editor-accordion" open>
					<summary>{$_("gameEditor.locationDescAccordion")}</summary>
					<div class="editor-accordion-content">
						<div class="field">
							<label for="location_des">{$_("gameEditor.descLabel")}</label>
							<textarea
								id="location_des"
								bind:value={activeLocation.description}
								rows="4"
								placeholder={$_("gameEditor.locationDescPlaceholder")}></textarea>
						</div>
					</div>
				</details>

				<details class="editor-accordion location-assets" open>
					<summary>{$_("gameEditor.locationBgAccordion")}</summary>
					<div class="editor-accordion-content">
						<div class="section-heading">
							<div><p>{$_("gameEditor.locationBgDesc")}</p></div>
							<button class="add-btn" onclick={handleAddBackground}>{$_("gameEditor.addBgBtn")}</button>
						</div>
						{#if backgroundAssets.filter((asset) => asset.locationId === activeLocation.id).length}
							<div class="background-asset-list">
								{#each backgroundAssets.filter((asset) => asset.locationId === activeLocation.id) as asset (asset.id)}
									<article class="background-asset-card">
										<div class="background-preview">
											{#if editorAssetPreviewUrl(assetRoot, asset.filePath)}<img
													src={editorAssetPreviewUrl(assetRoot, asset.filePath)}
													alt={asset.name}
												/>{:else}<span>▧</span>{/if}
										</div>
										<div class="background-fields">
											<label>{$_("gameEditor.bgNameLabel")}<input bind:value={asset.name} /></label><label
												>{$_("gameEditor.keyLabel")}<input bind:value={asset.key} /></label
											><label
												>{$_("gameEditor.filePathLabel")}<input
													bind:value={asset.filePath}
													placeholder="backgrounds/..."
												/></label
											>
											<div class="asset-field-row">
												<label
													>{$_("gameEditor.tabTime")}<select bind:value={asset.timeId}
														><option value={null}>{$_("gameEditor.allTimes")}</option
														>{#each timePeriods as period (period.id)}<option
																value={period.id}>{period.name}</option
															>{/each}</select
													></label
												><label
													>{$_("gameEditor.tabWeather")}<select bind:value={asset.weatherId}
														><option value={null}>{$_("gameEditor.allWeathers")}</option
														>{#each weatherTypes as weather (weather.id)}<option
																value={weather.id}>{weather.name}</option
															>{/each}</select
													></label
												>
											</div>
										</div>
										<button
											class="asset-remove"
											aria-label={$_("gameEditor.deleteBgAria")}
											onclick={() => removeBackground(asset.id)}>×</button
										>
									</article>
								{/each}
							</div>
						{:else}<div class="asset-empty">
								{$_("gameEditor.noLocationBgs")}
							</div>{/if}
					</div>
				</details>
				<details class="editor-accordion location-links" open>
					<summary>{$_("gameEditor.routesAccordion")}</summary>
					<div class="editor-accordion-content">
						<div class="section-heading">
							<div><p>{$_("gameEditor.routesDesc")}</p></div>
							<button class="add-btn" onclick={handleAddRoute} disabled={!canAddRoute()}
								>{$_("gameEditor.addRouteBtn")}</button
							>
						</div>
						{#if activeLocationRoutes.length}<div class="location-link-list">
								{#each activeLocationRoutes as route (route.index)}<article class="location-link-card">
										<div class="link-route">
											<label
												>{$_("gameEditor.fromLocationLabel")}<select bind:value={route.connection.fromId}
													>{#each locations as location (location.id)}<option
															value={location.id ?? ""}>{location.name}</option
														>{/each}</select
												></label
											><span>→</span><label
												>{$_("gameEditor.toLocationLabel")}<select bind:value={route.connection.toId}
													>{#each locations as location (location.id)}<option
															value={location.id ?? ""}>{location.name}</option
														>{/each}</select
												></label
											>
										</div>
										<div class="link-settings">
											<label
												>{$_("gameEditor.travelSecondsLabel")}<input
													type="number"
													min="0"
													step="10"
													bind:value={route.connection.travelSeconds}
												/></label
											><label
												>{$_("gameEditor.travelConditionLabel")}<input
													bind:value={route.connection.condition}
													placeholder={$_("gameEditor.optionalPlaceholder")}
												/></label
											><button
												class="asset-remove"
												aria-label={$_("gameEditor.deleteRouteAria")}
												onclick={() => handleDeleteRoute(route.index)}>×</button
											>
										</div>
									</article>{/each}
							</div>{:else}<div class="asset-empty">{$_("gameEditor.noRoutes")}</div>{/if}
					</div>
				</details>
				<details class="editor-accordion location-links" open>
					<summary>{$_("gameEditor.interactablesAccordion")}</summary>
					<div class="editor-accordion-content">
						<div class="section-heading">
							<div><p>{$_("gameEditor.interactablesDesc")}</p></div>
							<button class="add-btn" onclick={handleAddInteractable}>{$_("gameEditor.addInteractableBtn")}</button>
						</div>
						{#if activeLocationInteractables.length}<div class="location-interactable-list">
								{#each activeLocationInteractables as entry (entry.index)}<article
										class="location-interactable-card"
									>
										<span class="record-icon">✦</span><label
											>{$_("gameEditor.interactableNameLabel")}<input bind:value={entry.item.name} /></label
										><label
											>{$_("gameEditor.linkedEventLabel")}<select bind:value={entry.item.eventId}
												><option value={null}>{$_("gameEditor.noLinkOption")}</option
												>{#each events as event (event.id)}<option value={event.id}
														>{event.name}</option
													>{/each}</select
											></label
										><button
											class="asset-remove"
											aria-label={$_("gameEditor.deleteInteractableAria")}
											onclick={() => handleDeleteInteractable(entry.index)}>×</button
										>
									</article>{/each}
							</div>{:else}<div class="asset-empty">
								{$_("gameEditor.noInteractables")}
							</div>{/if}
					</div>
				</details>
			</div>
		{:else if activeZone}
			<div class="detail-header">
				<h3>{$_("gameEditor.editZoneHeading")} <strong>{activeZone.name}</strong></h3>
			</div>
			<div class="form-grid">
				<div class="field">
					<label for="zone_name">{$_("gameEditor.zoneNameLabel")}</label>
					<input id="zone_name" type="text" bind:value={activeZone.name} />
				</div>
			</div>
		{:else if activeMap}
			<div class="detail-header">
				<h3>{$_("gameEditor.editMapHeading")} <strong>{activeMap.name}</strong></h3>
			</div>
			<div class="form-grid">
				<div class="field">
					<label for="map_name">{$_("gameEditor.mapNameLabel")}</label>
					<input id="map_name" type="text" bind:value={activeMap.name} />
				</div>
				<div class="field">
					<label for="map_danger">{$_("gameEditor.dangerLevelLabel")}</label>
					<input id="map_danger" type="number" bind:value={activeMap.dangerLevel} min="0" max="10" />
				</div>
				<div class="field">
					<label for="map_default_weather_pool">{$_("gameEditor.defaultWeatherPoolLabel")}</label>
					<select id="map_default_weather_pool" bind:value={activeMap.defaultWeatherPoolId}>
						<option value={null}>{$_("gameEditor.noWeatherPool")}</option>
						{#if activeMap.defaultWeatherPoolId}<option value={activeMap.defaultWeatherPoolId}>{$_("gameEditor.currentWeatherPool")}</option>{/if}
					</select>
				</div>
			</div>
		{:else}
			<div class="no-selection">{$_("gameEditor.selectMapHierarchy")}</div>
		{/if}
	</div>
</div>

<style>
	.map-hierarchy-layout {
		display: grid;
		grid-template-columns: minmax(230px, 290px) minmax(0, 1fr);
		gap: 1rem;
		height: 100%;
		min-height: 550px;
	}
	
	.hierarchy-level .pane-header {
		padding: 0.7rem 0.8rem;
	}
	.hierarchy-level .scroll-list {
		max-height: 180px;
	}
	.location-row-copy {
		display: flex;
		flex-direction: column;
		gap: 0.18rem;
		min-width: 0;
		overflow: hidden;
	}
	.location-row-meta {
		overflow: hidden;
		color: var(--text-muted);
		font-size: 0.68rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}


	.pane-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.8rem;
		border-bottom: 1px solid var(--border);
		background: var(--surface);
	}

	.pane-header h4 {
		margin: 0;
		font-size: 0.85rem;
		color: var(--text);
	}

	.add-btn {
		background: var(--accent);
		color: var(--text-on-accent);
		border: none;
		padding: 0.3rem 0.6rem;
		border-radius: 4px;
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
	}
	.add-btn:disabled {
		background: var(--border-strong);
		opacity: 0.6;
		cursor: not-allowed;
	}
	.add-btn:not(:disabled):hover {
		background: var(--accent-hover);
	}

	.scroll-list {
		flex: 1;
		overflow-y: auto;
		padding: 0.4rem;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}


	.icon-del {
		background: transparent;
		border: none;
		color: var(--text-muted);
		font-size: 1rem;
		cursor: pointer;
		padding: 0 0.3rem;
		border-radius: 4px;
	}
	.icon-del:hover {
		background: var(--danger);
		color: var(--text-on-accent);
	}

	.empty-text {
		text-align: center;
		color: var(--text-faint);
		padding: 2rem 1rem;
		font-size: 0.8rem;
	}


	.detail-header {
		margin-bottom: 1.2rem;
		border-bottom: 1px solid var(--border);
		padding-bottom: 0.8rem;
	}
	.detail-header h3 {
		margin: 0;
		font-size: 1rem;
		color: var(--text);
	}
	.detail-header strong {
		color: var(--accent);
	}

	.form-grid {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.field label {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-muted);
	}
	.field input,
	.field textarea {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.6rem;
		color: var(--text);
		font-size: 0.95rem;
	}
	.field input:focus,
	.field textarea:focus {
		outline: none;
		border-color: var(--accent);
	}

	.no-selection {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--text-faint);
		font-size: 0.9rem;
		text-align: center;
	}
	.location-assets,
	.location-links {
		margin-top: 0.25rem;
	}
	.section-heading {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.8rem;
		margin-bottom: 0.8rem;
	}

	.section-heading p {
		margin: 0.25rem 0 0;
		color: var(--text-muted);
		font-size: 0.78rem;
	}
	.background-asset-list {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
		gap: 0.75rem;
	}
	.background-asset-card {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		align-items: stretch;
		padding: 0.7rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-2);
	}
	.background-preview {
		display: grid;
		place-items: center;
		width: 100%;
		aspect-ratio: 16/10;
		overflow: hidden;
		border-radius: var(--radius-md);
		background: var(--surface-active);
		color: var(--text-faint);
		font-size: 1.4rem;
	}
	.background-preview img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.background-fields {
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
		min-width: 0;
	}
	.background-fields label {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		color: var(--text-muted);
		font-size: 0.72rem;
		font-weight: 600;
	}
	.background-fields input,
	.background-fields select {
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
		padding: 0.48rem 0.6rem;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.8rem;
	}
	.asset-field-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.55rem;
	}
	.asset-remove {
		align-self: start;
		border: 0;
		background: transparent;
		color: var(--text-muted);
		font-size: 1.2rem;
		cursor: pointer;
	}
	.asset-remove:hover {
		color: var(--danger);
	}
	.asset-empty {
		padding: 1rem;
		border: 1px dashed var(--border-strong);
		border-radius: var(--radius-md);
		color: var(--text-muted);
		font-size: 0.8rem;
		text-align: center;
	}
	.location-link-list,
	.location-interactable-list {
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
	}
	.location-link-card,
	.location-interactable-card {
		position: relative;
		padding: 0.75rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-2);
	}
	.link-route {
		display: grid;
		grid-template-columns: 1fr 24px 1fr;
		gap: 0.55rem;
		align-items: end;
	}
	.link-route > span {
		padding-bottom: 0.55rem;
		color: var(--text-faint);
		text-align: center;
	}
	.link-settings {
		display: grid;
		grid-template-columns: minmax(100px, 0.5fr) minmax(0, 1.5fr) 24px;
		gap: 0.55rem;
		align-items: end;
		margin-top: 0.55rem;
	}
	.link-route label,
	.link-settings label,
	.location-interactable-card label {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		color: var(--text-muted);
		font-size: 0.7rem;
		font-weight: 600;
	}
	.link-route select,
	.link-settings input,
	.location-interactable-card input,
	.location-interactable-card select {
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
		padding: 0.48rem 0.55rem;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.78rem;
	}
	.location-interactable-card {
		display: grid;
		grid-template-columns: 32px minmax(0, 1fr) minmax(0, 1fr) 24px;
		gap: 0.6rem;
		align-items: center;
	}
	@media (max-width: 760px) {
		.map-hierarchy-layout {
			grid-template-columns: 1fr;
		}
	}
	@media (max-width: 640px) {
		.asset-field-row,
		.link-route {
			grid-template-columns: 1fr;
		}
		.link-route > span {
			display: none;
		}
		.link-settings {
			grid-template-columns: 1fr 24px;
		}
		.link-settings label:nth-child(2) {
			grid-column: 1/-1;
			grid-row: 2;
		}
		.location-interactable-card {
			grid-template-columns: 28px 1fr 24px;
		}
		.location-interactable-card label:nth-of-type(2) {
			grid-column: 2;
		}
	}
</style>
