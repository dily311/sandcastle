<script lang="ts">
	import type { GameLocationsInput, GameMapsInput, GameMapsZoneInput } from "$/lib/type/game";

	type Props = {
		maps: GameMapsInput[];
		zones: GameMapsZoneInput[];
		locations: GameLocationsInput[];
		gameId: string;
	};
	let { maps = $bindable([]), zones = $bindable([]), locations = $bindable([]), gameId }: Props = $props();

	// 현재 선택된 항목들의 인덱스 또는 ID 관리
	let selectedMapIndex = $state<number | null>(maps.length > 0 ? 0 : null);
	let selectedZoneIndex = $state<number | null>(null);
	let selectedLocationIndex = $state<number | null>(null);

	// 1. 현재 선택된 맵
	let activeMap = $derived(selectedMapIndex !== null && maps[selectedMapIndex] ? maps[selectedMapIndex] : null);

	// 2. 현재 선택된 맵에 속한 존(Zone)들 필터링
	let currentZones = $derived(activeMap ? zones.filter((z) => z.mapId === activeMap.id) : []);
	let activeZone = $derived(
		selectedZoneIndex !== null && currentZones[selectedZoneIndex] ? currentZones[selectedZoneIndex] : null,
	);

	// 3. 현재 선택된 존에 속한 로케이션(Location)들 필터링
	let currentLocations = $derived(activeZone ? locations.filter((l) => l.zoneId === activeZone.id) : []);
	let activeLocation = $derived(
		selectedLocationIndex !== null && currentLocations[selectedLocationIndex]
			? currentLocations[selectedLocationIndex]
			: null,
	);

	// --- 생성 및 삭제 핸들러 ---

	// 맵 추가
	function handleAddMap() {
		const newMap: GameMapsInput = {
			id: crypto.randomUUID(),
			gameId: gameId,
			name: "새로운 맵 영역",
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
			name: "새로운 존(Zone)",
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
			name: "새로운 로케이션",
			description: "",
			settingsJson: null,
		};
		locations.push(newLoc);
		selectedLocationIndex = currentLocations.length;
	}

	// 삭제 핸들러들
	function handleDeleteMap(index: number) {
		const targetMap = maps[index];
		if (!confirm(`"${targetMap.name}" 맵을 삭제하시겠습니까? 하위 존과 로케이션도 함께 관리해야 합니다.`)) return;

		// 연관된 존과 로케이션도 정리
		if (targetMap.id) {
			const zoneIdsToRemove = zones.filter((z) => z.mapId === targetMap.id).map((z) => z.id);
			zones = zones.filter((z) => z.mapId !== targetMap.id);
			locations = locations.filter((l) => l.zoneId && !zoneIdsToRemove.includes(l.zoneId));
		}

		maps.splice(index, 1);
		selectedMapIndex = maps.length > 0 ? 0 : null;
		selectedZoneIndex = null;
		selectedLocationIndex = null;
	}

	function handleDeleteZone(zoneId: string | null) {
		if (!zoneId || !confirm("이 존(Zone)을 삭제하시겠습니까? 하위 로케이션도 함께 삭제됩니다.")) return;

		zones = zones.filter((z) => z.id !== zoneId);
		locations = locations.filter((l) => l.zoneId !== zoneId);
		selectedZoneIndex = null;
		selectedLocationIndex = null;
	}

	function handleDeleteLocation(locId: string | null) {
		if (!locId || !confirm("이 로케이션을 삭제하시겠습니까?")) return;

		locations = locations.filter((l) => l.id !== locId);
		selectedLocationIndex = null;
	}
</script>

<div class="map-hierarchy-layout">
	<!-- 1단: 맵(Map) 선택/목록 영역 -->
	<div class="column-pane">
		<div class="pane-header">
			<h4>맵 (Maps)</h4>
			<button class="add-btn" onclick={handleAddMap}>+ 맵 추가</button>
		</div>
		<div class="scroll-list">
			{#each maps as map, idx (map.id || idx)}
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
				<div class="empty-text">맵이 없습니다.</div>
			{/each}
		</div>
	</div>

	<!-- 2단: 존(Zone) 선택/목록 영역 (선택된 맵이 있을 때만 활성) -->
	<div class="column-pane">
		<div class="pane-header">
			<h4>존 (Zones)</h4>
			<button class="add-btn" onclick={handleAddZone} disabled={!activeMap}>+ 존 추가</button>
		</div>
		<div class="scroll-list">
			{#if activeMap}
				{#each currentZones as zone, idx (zone.id || idx)}
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
					<div class="empty-text">등록된 존이 없습니다.</div>
				{/each}
			{:else}
				<div class="empty-text">맵을 먼저 선택해주세요.</div>
			{/if}
		</div>
	</div>

	<!-- 3단: 로케이션(Location) 선택/목록 영역 (선택된 존이 있을 때만 활성) -->
	<div class="column-pane">
		<div class="pane-header">
			<h4>로케이션 (Locations)</h4>
			<button class="add-btn" onclick={handleAddLocation} disabled={!activeZone}>+ 장소 추가</button>
		</div>
		<div class="scroll-list">
			{#if activeZone}
				{#each currentLocations as loc, idx (loc.id || idx)}
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="list-item"
						class:active={selectedLocationIndex === idx}
						onclick={() => (selectedLocationIndex = idx)}
					>
						<span class="title">{loc.name}</span>
						<button class="icon-del" onclick={() => handleDeleteLocation(loc.id)}>×</button>
					</div>
				{:else}
					<div class="empty-text">등록된 로케이션이 없습니다.</div>
				{/each}
			{:else}
				<div class="empty-text">존(Zone)을 먼저 선택해주세요.</div>
			{/if}
		</div>
	</div>

	<!-- 4단: 상세 편집 폼 (선택된 대상에 따라 동적으로 폼 변경) -->
	<div class="detail-pane">
		{#if activeLocation}
			<div class="detail-header">
				<h3>로케이션 편집: <strong>{activeLocation.name}</strong></h3>
			</div>
			<div class="form-grid">
				<div class="field">
					<label for="location_name">장소 이름</label>
					<input id="location_name" type="text" bind:value={activeLocation.name} />
				</div>
				<div class="field">
					<label for="location_des">설명</label>
					<textarea
						id="location_des"
						bind:value={activeLocation.description}
						rows="4"
						placeholder="장소 설명 입력..."></textarea>
				</div>
			</div>
		{:else if activeZone}
			<div class="detail-header">
				<h3>존(Zone) 편집: <strong>{activeZone.name}</strong></h3>
			</div>
			<div class="form-grid">
				<div class="field">
					<label for="zone_name">존 이름</label>
					<input id="zone_name" type="text" bind:value={activeZone.name} />
				</div>
			</div>
		{:else if activeMap}
			<div class="detail-header">
				<h3>맵 편집: <strong>{activeMap.name}</strong></h3>
			</div>
			<div class="form-grid">
				<div class="field">
					<label for="map_name">맵 이름</label>
					<input id="map_name" type="text" bind:value={activeMap.name} />
				</div>
				<div class="field">
					<label for="map_danger">위험도 (Danger Level)</label>
					<input id="map_danger" type="number" bind:value={activeMap.dangerLevel} min="0" max="10" />
				</div>
				<div class="field">
					<label for="map_default_wather_pool">기본 날씨 풀 ID</label>
					<input
						id="map_default_wather_pool"
						type="text"
						bind:value={activeMap.defaultWeatherPoolId}
						placeholder="선택 사항"
					/>
				</div>
			</div>
		{:else}
			<div class="no-selection">왼쪽에서 맵, 존, 혹은 로케이션을 선택하여 편집하세요.</div>
		{/if}
	</div>
</div>

<style>
	.map-hierarchy-layout {
		display: grid;
		grid-template-columns: 220px 220px 220px 1fr;
		gap: 1rem;
		height: 100%;
		min-height: 550px;
	}

	.column-pane,
	.detail-pane {
		background: #1e293b;
		border: 1px solid #334155;
		border-radius: 8px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.pane-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.8rem;
		border-bottom: 1px solid #334155;
		background: #0f172a;
	}

	.pane-header h4 {
		margin: 0;
		font-size: 0.85rem;
		color: #f8fafc;
	}

	.add-btn {
		background: #3b82f6;
		color: white;
		border: none;
		padding: 0.3rem 0.6rem;
		border-radius: 4px;
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
	}
	.add-btn:disabled {
		background: #475569;
		opacity: 0.6;
		cursor: not-allowed;
	}
	.add-btn:not(:disabled):hover {
		background: #2563eb;
	}

	.scroll-list {
		flex: 1;
		overflow-y: auto;
		padding: 0.4rem;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}

	.list-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.6rem 0.8rem;
		background: #0f172a;
		border-radius: 6px;
		cursor: pointer;
		border: 1px solid transparent;
		font-size: 0.85rem;
		color: #cbd5e1;
		transition: all 0.15s;
	}
	.list-item:hover {
		border-color: #475569;
	}
	.list-item.active {
		border-color: #38bdf8;
		background: #172554;
		color: #fff;
	}

	.icon-del {
		background: transparent;
		border: none;
		color: #94a3b8;
		font-size: 1rem;
		cursor: pointer;
		padding: 0 0.3rem;
		border-radius: 4px;
	}
	.icon-del:hover {
		background: #ef4444;
		color: white;
	}

	.empty-text {
		text-align: center;
		color: #64748b;
		padding: 2rem 1rem;
		font-size: 0.8rem;
	}

	.detail-pane {
		padding: 1.5rem;
		overflow-y: auto;
	}

	.detail-header {
		margin-bottom: 1.2rem;
		border-bottom: 1px solid #334155;
		padding-bottom: 0.8rem;
	}
	.detail-header h3 {
		margin: 0;
		font-size: 1rem;
		color: #f8fafc;
	}
	.detail-header strong {
		color: #38bdf8;
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
		color: #94a3b8;
	}
	.field input,
	.field textarea {
		background: #0f172a;
		border: 1px solid #334155;
		border-radius: 6px;
		padding: 0.6rem;
		color: #e2e8f0;
		font-size: 0.95rem;
	}
	.field input:focus,
	.field textarea:focus {
		outline: none;
		border-color: #38bdf8;
	}

	.no-selection {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: #64748b;
		font-size: 0.9rem;
		text-align: center;
	}
</style>
