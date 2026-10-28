<script lang="ts">
	import type { GameEventsInput } from "$/lib/type/game";

	type Props = {
		events: GameEventsInput[];
		gameId: string;
	};
	let { events = $bindable([]), gameId }: Props = $props();

	let selectedIndex = $state<number | null>(events.length > 0 ? 0 : null);
	let activeEvent = $derived(selectedIndex !== null && events[selectedIndex] ? events[selectedIndex] : null);

	function handleAddEvent() {
		const newEvent: GameEventsInput = {
			id: crypto.randomUUID(),
			gameId: gameId,
			eventType: "custom",
			triggerType: "auto",
			repeatPolicy: "once",
			name: "새로운 이벤트",
			description: "",
			requiresNpc: false,
			configJson: {},
			effectJson: {},
			cooldownSeconds: 0,
		};
		events.push(newEvent);
		selectedIndex = events.length - 1;
	}

	function handleDeleteEvent(index: number) {
		if (!confirm(`"${events[index].name}" 이벤트를 삭제하시겠습니까?`)) return;
		events.splice(index, 1);
		selectedIndex = events.length > 0 ? Math.max(0, index - 1) : null;
	}

	function toJsonString(obj: unknown): string {
		try {
			return JSON.stringify(obj, null, 2);
		} catch {
			return "{}";
		}
	}

	function fromJsonString(str: string, target: "configJson" | "effectJson") {
		if (!activeEvent) return;
		try {
			activeEvent[target] = JSON.parse(str);
		} catch {
			// 파싱 오류 무시
		}
	}
</script>

<div class="sub-editor-layout">
	<!-- 좌측: 이벤트 목록 -->
	<div class="item-list-pane">
		<div class="pane-header">
			<h4>이벤트 목록 ({events.length})</h4>
			<button class="add-btn" onclick={handleAddEvent}>+ 이벤트 추가</button>
		</div>
		<div class="scroll-list">
			{#each events as ev, idx (ev.id)}
				<div class="list-item" class:active={selectedIndex === idx} onclick={() => (selectedIndex = idx)}>
					<div class="item-info">
						<span class="title">{ev.name}</span>
						<span class="meta">유형: {ev.eventType}</span>
					</div>
					<button class="icon-del" onclick={() => handleDeleteEvent(idx)}>×</button>
				</div>
			{:else}
				<div class="empty-text">등록된 이벤트가 없습니다.</div>
			{/each}
		</div>
	</div>

	<!-- 우측: 상세 편집 폼 -->
	<div class="item-detail-pane">
		{#if activeEvent && selectedIndex !== null}
			<div class="detail-header">
				<h3>이벤트 상세 편집</h3>
				<span class="sub-id">ID: {activeEvent.id}</span>
			</div>
			<div class="form-grid">
				<div class="field-row-split">
					<div class="field">
						<label>이벤트 이름 (Name)</label>
						<input type="text" bind:value={activeEvent.name} />
					</div>
					<div class="field">
						<label>이벤트 유형 (Event Type)</label>
						<input
							type="text"
							bind:value={activeEvent.eventType}
							placeholder="예: combat, dialog, script"
						/>
					</div>
				</div>

				<div class="field">
					<label>설명 (Description)</label>
					<textarea bind:value={activeEvent.description} rows="2" placeholder="이벤트 설명..."></textarea>
				</div>

				<div class="field-row-split">
					<div class="field">
						<label>쿨다운 시간 (Seconds)</label>
						<input type="number" bind:value={activeEvent.cooldownSeconds} min="0" />
					</div>
					<div class="field checkbox-field">
						<label class="checkbox-label">
							<input type="checkbox" bind:checked={activeEvent.requiresNpc} />
							<span>NPC 필요 여부 (Requires NPC)</span>
						</label>
					</div>
				</div>

				<div class="field-row-split">
					<div class="field">
						<label>설정 JSON (Config JSON)</label>
						<textarea
							class="code-font"
							rows="4"
							value={toJsonString(activeEvent.configJson)}
							oninput={(e) => fromJsonString(e.currentTarget.value, "configJson")}></textarea>
					</div>
					<div class="field">
						<label>효과 JSON (Effect JSON)</label>
						<textarea
							class="code-font"
							rows="4"
							value={toJsonString(activeEvent.effectJson)}
							oninput={(e) => fromJsonString(e.currentTarget.value, "effectJson")}></textarea>
					</div>
				</div>
			</div>
		{:else}
			<div class="no-selection">왼쪽에서 이벤트를 선택하거나 새로 추가하세요.</div>
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
	.sub-id {
		font-size: 0.75rem;
		color: #64748b;
		font-family: monospace;
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
	.code-font {
		font-family: monospace;
		font-size: 0.85rem;
	}
	.checkbox-field {
		display: flex;
		align-items: flex-end;
		padding-bottom: 0.4rem;
	}
	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		font-size: 0.9rem;
		color: #e2e8f0;
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
