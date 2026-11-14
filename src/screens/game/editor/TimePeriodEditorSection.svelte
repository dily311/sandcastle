<script lang="ts">
	import type { GameTimePeriodsInput } from "$/lib/type/game";

	type Props = {
		timePeriods: GameTimePeriodsInput[];
		gameId: string;
	};
	let { timePeriods = $bindable([]), gameId }: Props = $props();

	let selectedIndex = $state<number | null>(timePeriods.length > 0 ? 0 : null);
	let searchQuery = $state("");
	let filteredPeriods = $derived(timePeriods.map((tp, index) => ({ tp, index })).filter(({ tp }) => `${tp.name} ${tp.startMinute} ${tp.endMinute}`.toLowerCase().includes(searchQuery.trim().toLowerCase())));
	let activePeriod = $derived(
		selectedIndex !== null && timePeriods[selectedIndex] ? timePeriods[selectedIndex] : null,
	);

	function handleAddTimePeriod() {
		const newPeriod: GameTimePeriodsInput = {
			id: crypto.randomUUID(),
			gameId: gameId,
			name: "새로운 시간대",
			sortOrder: timePeriods.length,
			startMinute: 0,
			endMinute: 360,
		};
		timePeriods.push(newPeriod);
		selectedIndex = timePeriods.length - 1;
	}

	function handleDeleteTimePeriod(index: number) {
		if (!confirm(`"${timePeriods[index].name}" 시간대를 삭제하시겠습니까?`)) return;
		timePeriods.splice(index, 1);
		selectedIndex = timePeriods.length > 0 ? Math.max(0, index - 1) : null;
	}
</script>

<div class="sub-editor-layout">
	<!-- 좌측: 시간대 목록 -->
	<div class="item-list-pane">
		<label class="list-search"><span aria-hidden="true">⌕</span><input aria-label="시간대 검색" placeholder="시간대 검색" bind:value={searchQuery} /></label>
		<div class="pane-header">
			<h4>시간대 목록 ({timePeriods.length})</h4>
			<button class="add-btn" onclick={handleAddTimePeriod}>+ 시간대 추가</button>
		</div>
		<div class="scroll-list">
			{#each filteredPeriods as row (row.tp.id || row.index)}
				{@const tp = row.tp}{@const idx = row.index}
				<div role="none" class="list-item" class:active={selectedIndex === idx} onclick={() => (selectedIndex = idx)}>
					<div class="item-info">
						<span class="title">{tp.name}</span>
						<span class="meta">{tp.startMinute}분 ~ {tp.endMinute}분</span>
					</div>
					<button class="icon-del" onclick={() => handleDeleteTimePeriod(idx)}>×</button>
				</div>
			{:else}
				<div class="empty-text">등록된 시간대가 없습니다.</div>
			{/each}
		</div>
	</div>

	<!-- 우측: 상세 편집 폼 -->
	<div class="item-detail-pane">
		{#if activePeriod && selectedIndex !== null}
			<div class="detail-header">
				<h3>시간대 상세 편집</h3>
			</div>
			<details class="editor-accordion" open><summary>시간대 설정</summary><div class="editor-accordion-content"><div class="form-grid">
				<div class="field">
					<label for="time_name">시간대 이름 (Name)</label>
					<input id="time_name" type="text" bind:value={activePeriod.name} placeholder="예: 새벽, 아침, 낮, 밤" />
				</div>
				<div class="field-row-split">
					<div class="field">
						<label for="time_s_minute">시작 분 (Start Minute)</label>
						<input id="time_s_minute" type="number" bind:value={activePeriod.startMinute} min="0" max="1440" />
						<span class="field-hint">하루 1440분 기준 (예: 0 = 00:00)</span>
					</div>
					<div class="field">
						<label for="time_e_minute">종료 분 (End Minute)</label>
						<input id="time_e_minute" type="number" bind:value={activePeriod.endMinute} min="0" max="1440" />
						<span class="field-hint">예: 360 = 06:00</span>
					</div>
				</div>
				<div class="field">
					<label for="time_sort">정렬 순서 (Sort Order)</label>
					<input id="time_sort" type="number" bind:value={activePeriod.sortOrder} />
				</div>
			</div></div></details>
		{:else}
			<div class="no-selection">왼쪽에서 시간대를 선택하거나 새로 추가하세요.</div>
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
	.field input {
		background: #0f172a;
		border: 1px solid #334155;
		border-radius: 6px;
		padding: 0.6rem;
		color: #e2e8f0;
		font-size: 0.95rem;
	}
	.field input:focus {
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
