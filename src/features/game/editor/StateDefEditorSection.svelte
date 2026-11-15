<script lang="ts">
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	/** 지우기 전에 앱 확인 창으로 물어봄 */
	const ask = (message: string) => confirmAction({ title: "삭제", message, confirmLabel: "삭제", danger: true });
	import type { GameStateDefInput } from "$features/game/types";
	import IconButton from "$/shared/ui/IconButton.svelte";
	import { Plus, Search, X } from "@lucide/svelte";
	import Input from "$/shared/ui/Input.svelte";

	type Props = {
		stateDefs: GameStateDefInput[];
		gameId: string;
	};
	let { stateDefs = $bindable([]), gameId }: Props = $props();

	let selectedIndex = $state<number | null>(stateDefs.length > 0 ? 0 : null);
	let searchQuery = $state("");
	let filteredStateDefs = $derived(stateDefs.map((stateDef, index) => ({ stateDef, index })).filter(({ stateDef }) => `${stateDef.key} ${stateDef.type} ${stateDef.description}`.toLowerCase().includes(searchQuery.trim().toLowerCase())));
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

	async function handleDeleteStateDef(index: number) {
		if (!await ask(`"${stateDefs[index].key}" 상태 정의를 삭제하시겠습니까?`)) return;
		stateDefs.splice(index, 1);
		selectedIndex = stateDefs.length > 0 ? Math.max(0, index - 1) : null;
	}
</script>

<div class="sub-editor-layout">
	<!-- 좌측: 상태 목록 -->
	<div class="item-list-pane">
		<div class="pane-header">
			<Input bind:value={searchQuery} placeholder="변수 검색" aria-label="변수 검색">
				{#snippet leading()}<Search />{/snippet}
			</Input>
			<IconButton label="변수 추가" size="md" onclick={handleAddStateDef}><Plus /></IconButton>
		</div>
		<div class="scroll-list">
			{#each filteredStateDefs as row (row.stateDef.id)}
				{@const ev = row.stateDef}
				{@const idx = row.index}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="list-item" class:active={selectedIndex === idx} onclick={() => (selectedIndex = idx)}>
					<div class="item-info">
						<span class="title"><code>{ev.key}</code></span>
						<span class="type">타입: {ev.type}</span>
					</div>
					<IconButton class="icon-del" size="sm" label="변수" onclick={() => handleDeleteStateDef(idx)}><X /></IconButton>
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
			</div>
			<details class="editor-accordion" open><summary>변수 설정</summary><div class="editor-accordion-content"><div class="form-grid">
				<div class="field">
					<label for="var_key">변수 키 (Key)</label>
					<input id="var_key" type="text" bind:value={activeState.key} />
				</div>
				<div class="field">
					<label for="var_type">데이터 타입 (Type)</label>
					<select id="var_type" bind:value={activeState.type}>
						<option value="string">string (문자열)</option>
						<option value="number">number (숫자)</option>
						<option value="boolean">boolean (참/거짓)</option>
					</select>
				</div>
				<div class="field">
					<label for="var_default">초기값 (Default Value)</label>
					<input id="var_default" type="text" bind:value={activeState.defaultValue} />
				</div>
				<div class="field">
					<label for="var_des">설명 (Description)</label>
					<textarea id="var_des" bind:value={activeState.description} rows="3"></textarea>
				</div>
			</div></div></details>
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
		height: 100%;
		min-height: 500px;
	}
	.item-list-pane,
	.item-detail-pane {

		display: flex;
		flex-direction: column;
		overflow: hidden;
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
	.field select,
	.field textarea {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.6rem;
		color: var(--text);
		font-size: 0.95rem;
	}
	.field input:focus,
	.field select:focus,
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
		font-size: 0.95rem;
		text-align: center;
	}
</style>
