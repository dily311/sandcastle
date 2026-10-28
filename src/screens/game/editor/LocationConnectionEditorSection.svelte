<script lang="ts">
	import type { GameLocationConnectionsInput, GameLocationsInput } from "$/lib/type/game";

	type Props = {
		connections: GameLocationConnectionsInput[];
		locations: GameLocationsInput[];
	};
	let { connections = $bindable([]), locations = $bindable([]) }: Props = $props();

	let selectedIndex = $state<number | null>(connections.length > 0 ? 0 : null);
	let activeConnection = $derived(
		selectedIndex !== null && connections[selectedIndex] ? connections[selectedIndex] : null,
	);

	// 로케이션 ID를 이름으로 쉽게 매핑하기 위한 맵
	let locationMap = $derived(new Map(locations.map((l) => [l.id, l.name])));

	function handleAddConnection() {
		if (locations.length < 2) {
			alert("연결을 설정하려면 최소 2개 이상의 로케이션이 필요합니다.");
			return;
		}

		const defaultFrom = locations[0]?.id || "";
		const defaultTo = locations[1]?.id || "";

		const newConnection: GameLocationConnectionsInput = {
			fromId: defaultFrom,
			toId: defaultTo,
			travelSeconds: 60, // 기본 이동 시간 (60초)
			condition: null,
			settingsJson: null,
		};

		connections.push(newConnection);
		selectedIndex = connections.length - 1;
	}

	function handleDeleteConnection(index: number) {
		if (!confirm("이 이동 경로 연결을 삭제하시겠습니까?")) return;
		connections.splice(index, 1);
		selectedIndex = connections.length > 0 ? Math.max(0, index - 1) : null;
	}
</script>

<div class="sub-editor-layout">
	<!-- 좌측: 연결 경로 목록 -->
	<div class="item-list-pane">
		<div class="pane-header">
			<h4>이동 경로 목록 ({connections.length})</h4>
			<button class="add-btn" onclick={handleAddConnection}>+ 경로 추가</button>
		</div>
		<div class="scroll-list">
			{#each connections as conn, index (conn.fromId + "->" + conn.toId)}
				{@const fromName = locationMap.get(conn.fromId) || "알 수 없는 장소"}
				{@const toName = locationMap.get(conn.toId) || "알 수 없는 장소"}
				<div class="list-item" class:active={selectedIndex === index} onclick={() => (selectedIndex = index)}>
					<div class="item-info">
						<span class="route-title">{fromName} ➔ {toName}</span>
						<span class="route-meta">이동 시간: {conn.travelSeconds}초</span>
					</div>
				</div>
			{:else}
				<div class="empty-text">설정된 이동 경로가 없습니다.</div>
			{/each}
		</div>
	</div>

	<!-- 우측: 선택된 경로 상세 편집 폼 -->
	<div class="item-detail-pane">
		{#if activeConnection && selectedIndex !== null}
			<div class="detail-header">
				<h3>이동 경로 상세 설정</h3>
				<button class="delete-btn" onclick={() => handleDeleteConnection(selectedIndex!)}>경로 삭제</button>
			</div>
			<div class="form-grid">
				<div class="field-row-split">
					<div class="field">
						<label>출발지 (From Location)</label>
						<select bind:value={activeConnection.fromId}>
							{#each locations as loc}
								<option value={loc.id}>{loc.name}</option>
							{/each}
						</select>
					</div>
					<div class="field">
						<label>도착지 (To Location)</label>
						<select bind:value={activeConnection.toId}>
							{#each locations as loc}
								<option value={loc.id}>{loc.name}</option>
							{/each}
						</select>
					</div>
				</div>

				<div class="field">
					<label>이동 소요 시간 (Travel Seconds)</label>
					<input type="number" bind:value={activeConnection.travelSeconds} min="0" step="10" />
					<span class="field-hint">초 단위로 입력하세요 (예: 60초 = 1분)</span>
				</div>

				<div class="field">
					<label>이동 조건식 (Condition / Expression)</label>
					<input
						type="text"
						bind:value={activeConnection.condition}
						placeholder="예: has_key_room_a == true (선택 사항)"
					/>
				</div>
			</div>
		{:else}
			<div class="no-selection">편집할 경로를 선택하거나 새로 추가해주세요.</div>
		{/if}
	</div>
</div>

<style>
	.sub-editor-layout {
		display: grid;
		grid-template-columns: 320px 1fr;
		gap: 1.5rem;
		height: 100%;
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
	.item-info .route-title {
		font-weight: 600;
		font-size: 0.9rem;
		color: #f8fafc;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.item-info .route-meta {
		font-size: 0.75rem;
		color: #94a3b8;
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
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.2rem;
		border-bottom: 1px solid #334155;
		padding-bottom: 0.8rem;
	}
	.detail-header h3 {
		margin: 0;
		font-size: 1.1rem;
		color: #f8fafc;
	}
	.delete-btn {
		background: #ef4444;
		color: white;
		border: none;
		padding: 0.4rem 0.8rem;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}
	.delete-btn:hover {
		background: #dc2626;
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
