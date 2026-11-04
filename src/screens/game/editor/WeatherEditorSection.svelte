<script lang="ts">
	import type { GameMapsInput, GameWeatherTransitionsInput, GameWeatherTypesInput } from "$/lib/type/game";

	type Props = {
		weatherTypes: GameWeatherTypesInput[];
		weatherTransitions: GameWeatherTransitionsInput[];
		maps: GameMapsInput[];
		gameId: string;
	};
	let { weatherTypes = $bindable([]), weatherTransitions = $bindable([]), maps = [], gameId }: Props = $props();

	let activeSubTab = $state<"types" | "transitions">("types");
	let selectedTypeIndex = $state<number | null>(weatherTypes.length > 0 ? 0 : null);
	let selectedTransitionIndex = $state<number | null>(null);

	let activeWeatherType = $derived(
		selectedTypeIndex !== null && weatherTypes[selectedTypeIndex] ? weatherTypes[selectedTypeIndex] : null,
	);
	let activeTransition = $derived(
		selectedTransitionIndex !== null && weatherTransitions[selectedTransitionIndex]
			? weatherTransitions[selectedTransitionIndex]
			: null,
	);

	// --- 날씨 타입(Weather Type) 핸들러 ---
	function handleAddWeatherType() {
		const newType: GameWeatherTypesInput = {
			id: crypto.randomUUID(),
			gameId: gameId,
			name: "새로운 날씨",
			encounterRateModifier: 1.0,
			visibilityModifier: 1.0,
		};
		weatherTypes.push(newType);
		selectedTypeIndex = weatherTypes.length - 1;
	}

	function handleDeleteWeatherType(index: number) {
		if (!confirm(`"${weatherTypes[index].name}" 날씨를 삭제하시겠습니까?`)) return;
		weatherTypes.splice(index, 1);
		selectedTypeIndex = weatherTypes.length > 0 ? Math.max(0, index - 1) : null;
	}

	// --- 날씨 전환(Weather Transition) 핸들러 ---
	function handleAddTransition() {
		if (weatherTypes.length < 1) {
			alert("날씨 전환을 설정하려면 먼저 날씨 종류를 1개 이상 추가해주세요.");
			return;
		}
		const defaultWeatherId = weatherTypes[0]?.id || "";
		const defaultMapId = maps[0]?.id || "";

		const newTrans: GameWeatherTransitionsInput = {
			fromWeatherId: defaultWeatherId,
			toWeatherId: defaultWeatherId,
			mapId: defaultMapId,
			probability: 0.5,
		};
		weatherTransitions.push(newTrans);
		selectedTransitionIndex = weatherTransitions.length - 1;
	}

	function handleDeleteTransition(index: number) {
		if (!confirm("이 날씨 전환 규칙을 삭제하시겠습니까?")) return;
		weatherTransitions.splice(index, 1);
		selectedTransitionIndex = weatherTransitions.length > 0 ? Math.max(0, index - 1) : null;
	}

	// ID로 날씨 이름 찾기 헬퍼
	let weatherMap = $derived(new Map(weatherTypes.map((w) => [w.id, w.name])));
	let mapNameMap = $derived(new Map(maps.map((m) => [m.id, m.name])));
</script>

<div class="weather-editor-container">
	<!-- 내부 서브 탭 (날씨 종류 vs 전환 규칙) -->
	<div class="sub-tabs">
		<button class:active={activeSubTab === "types"} onclick={() => (activeSubTab = "types")}>
			날씨 종류 ({weatherTypes.length})
		</button>
		<button class:active={activeSubTab === "transitions"} onclick={() => (activeSubTab = "transitions")}>
			날씨 전환 규칙 ({weatherTransitions.length})
		</button>
	</div>

	<div class="sub-editor-layout">
		{#if activeSubTab === "types"}
			<!-- 1단: 날씨 종류 목록 -->
			<div class="item-list-pane">
				<div class="pane-header">
					<h4>날씨 목록</h4>
					<button class="add-btn" onclick={handleAddWeatherType}>+ 날씨 추가</button>
				</div>
				<div class="scroll-list">
					{#each weatherTypes as wt, idx (wt.id || idx)}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="list-item"
							class:active={selectedTypeIndex === idx}
							onclick={() => (selectedTypeIndex = idx)}
						>
							<div class="item-info">
								<span class="title">{wt.name}</span>
								<span class="meta">조우 보정: {wt.encounterRateModifier}x</span>
							</div>
							<button class="icon-del" onclick={() => handleDeleteWeatherType(idx)}>×</button>
						</div>
					{:else}
						<div class="empty-text">등록된 날씨가 없습니다.</div>
					{/each}
				</div>
			</div>

			<!-- 2단: 날씨 종류 상세 편집 폼 -->
			<div class="item-detail-pane">
				{#if activeWeatherType && selectedTypeIndex !== null}
					<div class="detail-header">
						<h3>날씨 상세 편집</h3>
					</div>
					<div class="form-grid">
						<div class="field">
							<label for="weather_name">날씨 이름 (Name)</label>
							<input id="weather_name" type="text" bind:value={activeWeatherType.name} />
						</div>
						<div class="field-row-split">
							<div class="field">
								<label for="weather_rate">조우율 보정 (Encounter Rate Modifier)</label>
								<input
									id="weather_rate"
									type="number"
									step="0.1"
									bind:value={activeWeatherType.encounterRateModifier}
								/>
								<span class="field-hint">기본값 1.0 (예: 1.5배 증가)</span>
							</div>
							<div class="field">
								<label for="weather_visibility">시야각 보정 (Visibility Modifier)</label>
								<input
									id="weather_visibility"
									type="number"
									step="0.1"
									bind:value={activeWeatherType.visibilityModifier}
								/>
								<span class="field-hint">기본값 1.0 (예: 안개 시 0.5)</span>
							</div>
						</div>
					</div>
				{:else}
					<div class="no-selection">왼쪽에서 날씨를 선택하거나 새로 추가하세요.</div>
				{/if}
			</div>
		{:else}
			<!-- 1단: 날씨 전환 규칙 목록 -->
			<div class="item-list-pane">
				<div class="pane-header">
					<h4>전환 규칙 목록</h4>
					<button class="add-btn" onclick={handleAddTransition}>+ 규칙 추가</button>
				</div>
				<div class="scroll-list">
					{#each weatherTransitions as tr, idx (idx)}
						{@const fromName = weatherMap.get(tr.fromWeatherId) || "지정 안됨"}
						{@const toName = weatherMap.get(tr.toWeatherId) || "지정 안됨"}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="list-item"
							class:active={selectedTransitionIndex === idx}
							onclick={() => (selectedTransitionIndex = idx)}
						>
							<div class="item-info">
								<span class="title">{fromName} ➔ {toName}</span>
								<span class="meta">확률: {tr.probability * 100}%</span>
							</div>
							<button class="icon-del" onclick={() => handleDeleteTransition(idx)}>×</button>
						</div>
					{:else}
						<div class="empty-text">등록된 전환 규칙이 없습니다.</div>
					{/each}
				</div>
			</div>

			<!-- 2단: 전환 규칙 상세 편집 폼 -->
			<div class="item-detail-pane">
				{#if activeTransition && selectedTransitionIndex !== null}
					<div class="detail-header">
						<h3>날씨 전환 규칙 편집</h3>
					</div>
					<div class="form-grid">
						<div class="field-row-split">
							<div class="field">
								<label for="weather_from">시작 날씨 (From Weather)</label>
								<select id="weather_from" bind:value={activeTransition.fromWeatherId}>
									{#each weatherTypes as wt}
										<option value={wt.id}>{wt.name}</option>
									{/each}
								</select>
							</div>
							<div class="field">
								<label for="weather_to">전환될 날씨 (To Weather)</label>
								<select id="weather_to" bind:value={activeTransition.toWeatherId}>
									{#each weatherTypes as wt}
										<option value={wt.id}>{wt.name}</option>
									{/each}
								</select>
							</div>
						</div>

						<div class="field-row-split">
							<div class="field">
								<label for="weather_target_map">적용 맵 (Target Map)</label>
								<select id="weather_target_map" bind:value={activeTransition.mapId}>
									{#each maps as m}
										<option value={m.id}>{m.name}</option>
									{/each}
								</select>
							</div>
							<div class="field">
								<label for="weather_probability">전환 확률 (Probability: 0 ~ 1)</label>
								<input
									id="weather_probability"
									type="number"
									step="0.05"
									min="0"
									max="1"
									bind:value={activeTransition.probability}
								/>
								<span class="field-hint">0.5 = 50% 확률</span>
							</div>
						</div>
					</div>
				{:else}
					<div class="no-selection">왼쪽에서 전환 규칙을 선택하거나 새로 추가하세요.</div>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.weather-editor-container {
		display: flex;
		flex-direction: column;
		height: 100%;
		gap: 1rem;
	}

	.sub-tabs {
		display: flex;
		gap: 0.5rem;
		background: #0f172a;
		padding: 0.4rem;
		border-radius: 6px;
		border: 1px solid #334155;
	}
	.sub-tabs button {
		background: transparent;
		border: none;
		color: #94a3b8;
		padding: 0.4rem 1rem;
		font-size: 0.85rem;
		font-weight: 600;
		cursor: pointer;
		border-radius: 4px;
	}
	.sub-tabs button:hover {
		color: #f8fafc;
	}
	.sub-tabs button.active {
		background: #334155;
		color: #38bdf8;
	}

	.sub-editor-layout {
		display: grid;
		grid-template-columns: 320px 1fr;
		gap: 1.5rem;
		flex: 1;
		min-height: 500px;
	}
	.item-list-pane,
	.item-detail-pane {
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
		padding: 1rem;
		border-bottom: 1px solid #334155;
		background: #0f172a;
	}
	.pane-header h4 {
		margin: 0;
		font-size: 0.95rem;
		color: #f8fafc;
	}
	.add-btn {
		background: #3b82f6;
		color: white;
		border: none;
		padding: 0.4rem 0.8rem;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}
	.add-btn:hover {
		background: #2563eb;
	}

	.scroll-list {
		flex: 1;
		overflow-y: auto;
		padding: 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.list-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1rem;
		background: #0f172a;
		border-radius: 6px;
		cursor: pointer;
		border: 1px solid transparent;
		transition: all 0.15s;
	}
	.list-item:hover {
		border-color: #475569;
	}
	.list-item.active {
		border-color: #38bdf8;
		background: #172554;
	}

	.item-info {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		overflow: hidden;
	}
	.item-info .title {
		font-weight: 600;
		font-size: 0.9rem;
		color: #f8fafc;
	}
	.item-info .meta {
		font-size: 0.75rem;
		color: #94a3b8;
	}

	.icon-del {
		background: transparent;
		border: none;
		color: #94a3b8;
		font-size: 1rem;
		cursor: pointer;
		padding: 0.3rem;
		border-radius: 4px;
	}
	.icon-del:hover {
		background: #ef4444;
		color: white;
	}
	.empty-text {
		text-align: center;
		color: #64748b;
		padding: 2rem;
		font-size: 0.85rem;
	}

	.item-detail-pane {
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
		font-size: 1.1rem;
		color: #f8fafc;
	}

	.form-grid {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.field-row-split {
		display: grid;
		grid-template-columns: 1fr 1fr;
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
	.field select {
		background: #0f172a;
		border: 1px solid #334155;
		border-radius: 6px;
		padding: 0.6rem;
		color: #e2e8f0;
		font-size: 0.95rem;
	}
	.field input:focus,
	.field select:focus {
		outline: none;
		border-color: #38bdf8;
	}
	.field-hint {
		font-size: 0.75rem;
		color: #64748b;
	}

	.no-selection {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: #64748b;
		font-size: 0.95rem;
		text-align: center;
	}
</style>
