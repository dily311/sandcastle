<script lang="ts">
	import type { GameStateDefInput } from "$/lib/type/game";

	type Props = {
		stateDefs: GameStateDefInput[];
		gameId: string;
	};
	let { stateDefs = $bindable([]), gameId }: Props = $props();

	let selectedIndex = $state<number | null>(stateDefs.length > 0 ? 0 : null);
	let activeState = $derived(selectedIndex !== null && stateDefs[selectedIndex] ? stateDefs[selectedIndex] : null);

	function handleAddStateDef() {
		const newState: GameStateDefInput = {
			id: crypto.randomUUID(),
			gameId: gameId,
			key: `variable_${Date.now()}`,
			type: "string", // 기본 타입 예시 (string, number, boolean 등)
			defaultValue: "",
			description: "",
			configJson: "",
			createdAt: Date.now(),
			updatedAt: Date.now(),
		};
		stateDefs.push(newState);
		selectedIndex = stateDefs.length - 1;
	}

	function handleDeleteStateDef(index: number) {
		if (!confirm(`"${stateDefs[index].key}" 상태 정의를 삭제하시겠습니까?`)) return;
		stateDefs.splice(index, 1);
		selectedIndex = stateDefs.length > 0 ? Math.max(0, index - 1) : null;
	}
</script>

<div class="sub-editor-layout">
	<!-- 좌측: 상태 목록 -->
	<div class="item-list-pane">
		<div class="pane-header">
			<h4>상태 정의 목록 ({stateDefs.length})</h4>
			<button class="add-btn" onclick={handleAddStateDef}>+ 상태 추가</button>
		</div>
		<div class="scroll-list">
			{#each stateDefs as stateDef, index (stateDef.id)}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="list-item" class:active={selectedIndex === index} onclick={() => (selectedIndex = index)}>
					<div class="item-info">
						<span class="title"><code>{stateDef.key}</code></span>
						<span class="type">타입: {stateDef.type}</span>
					</div>
				</div>
			{:else}
				<div class="empty-text">등록된 상태 정의가 없습니다.</div>
			{/each}
		</div>
	</div>

	<!-- 우측: 상세 편집 폼 -->
	<div class="item-detail-pane">
		{#if activeState && selectedIndex !== null}
			<div class="detail-header">
				<h3>상태 정의 상세 편집</h3>
				<button class="delete-btn" onclick={() => handleDeleteStateDef(selectedIndex!)}>삭제</button>
			</div>
			<div class="form-grid">
				<div class="field">
					<label>변수 키 (Key)</label>
					<input type="text" bind:value={activeState.key} />
				</div>
				<div class="field">
					<label>데이터 타입 (Type)</label>
					<select bind:value={activeState.type}>
						<option value="string">string (문자열)</option>
						<option value="number">number (숫자)</option>
						<option value="boolean">boolean (참/거짓)</option>
					</select>
				</div>
				<div class="field">
					<label>초기값 (Default Value)</label>
					<input type="text" bind:value={activeState.defaultValue} />
				</div>
				<div class="field">
					<label>설명 (Description)</label>
					<textarea bind:value={activeState.description} rows="3"></textarea>
				</div>
			</div>
		{:else}
			<div class="no-selection">편집할 상태 정의를 선택하거나 새로 추가해주세요.</div>
		{/if}
	</div>
</div>

<style>
	/* 씬 에디터와 동일한 스타일 공유 테마 */
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
	}
	.item-info .title code {
		color: #38bdf8;
		background: #1e293b;
		padding: 0.1rem 0.3rem;
		border-radius: 4px;
		font-size: 0.85rem;
	}
	.item-info .type {
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
	.field select,
	.field textarea {
		background: #0f172a;
		border: 1px solid #334155;
		border-radius: 6px;
		padding: 0.6rem;
		color: #e2e8f0;
		font-size: 0.95rem;
	}
	.field input:focus,
	.field select:focus,
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
		font-size: 0.95rem;
		text-align: center;
	}
</style>
