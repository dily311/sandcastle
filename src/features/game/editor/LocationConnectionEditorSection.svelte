<script lang="ts">
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	/** 지우기 전에 앱 확인 창으로 물어봄 */
	const ask = (message: string) => confirmAction({ title: "삭제", message, confirmLabel: "삭제", danger: true });
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
			toasts.info("연결을 설정하려면 최소 2개 이상의 로케이션이 필요합니다.");
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

	async function handleDeleteConnection(index: number) {
		if (!await ask("이 이동 경로 연결을 삭제하시겠습니까?")) return;
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
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
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
						<label for="from_location">출발지 (From Location)</label>
						<select id="from_location" bind:value={activeConnection.fromId}>
							{#each locations as loc}
								<option value={loc.id}>{loc.name}</option>
							{/each}
						</select>
					</div>
					<div class="field">
						<label for="to_location">도착지 (To Location)</label>
						<select id="to_location" bind:value={activeConnection.toId}>
							{#each locations as loc}
								<option value={loc.id}>{loc.name}</option>
							{/each}
						</select>
					</div>
				</div>

				<div class="field">
					<label for="trabel_seconds">이동 소요 시간 (Travel Seconds)</label>
					<input id="trabel_seconds" type="number" bind:value={activeConnection.travelSeconds} min="0" step="10" />
					<span class="field-hint">초 단위로 입력하세요 (예: 60초 = 1분)</span>
				</div>

				<div class="field">
					<label for="trabel_condition">이동 조건식 (Condition / Expression)</label>
					<input
						id="trabel_condition"
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
		height: 100%;
		min-height: 500px;
	}
	.item-list-pane,
	.item-detail-pane {

		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.pane-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		border-bottom: 1px solid var(--border);
		background: var(--surface);
	}
	.pane-header h4 {
		margin: 0;
		font-size: 0.95rem;
		color: var(--text);
	}
	.add-btn {
		background: var(--accent);
		color: var(--text-on-accent);
		border: none;
		padding: 0.4rem 0.8rem;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}
	.add-btn:hover {
		background: var(--accent-hover);
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
		background: var(--surface);
		border-radius: 6px;
		cursor: pointer;
		border: 1px solid transparent;
		transition: all 0.15s;
	}
	.list-item:hover {
		border-color: var(--border-strong);
	}
	.list-item.active {
		border-color: var(--accent);
		background: var(--accent-soft);
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
		color: var(--text);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.item-info .route-meta {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
	.empty-text {
		text-align: center;
		color: var(--text-faint);
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
		border-bottom: 1px solid var(--border);
		padding-bottom: 0.8rem;
	}
	.detail-header h3 {
		margin: 0;
		font-size: 1.1rem;
		color: var(--text);
	}
	.delete-btn {
		background: var(--danger);
		color: var(--text-on-accent);
		border: none;
		padding: 0.4rem 0.8rem;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}
	.delete-btn:hover {
		background: color-mix(in oklab, var(--danger) 85%, black);
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
		color: var(--text-muted);
	}
	.field input,
	.field select {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.6rem;
		color: var(--text);
		font-size: 0.95rem;
	}
	.field input:focus,
	.field select:focus {
		outline: none;
		border-color: var(--accent);
	}
	.field-hint {
		font-size: 0.75rem;
		color: var(--text-faint);
	}

	.no-selection {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--text-faint);
		font-size: 0.95rem;
		text-align: center;
	}
</style>
