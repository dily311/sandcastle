<script lang="ts">
	import { _ } from "svelte-i18n";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	/** 지우기 전에 앱 확인 창으로 물어봄 */
	const ask = (message: string) => confirmAction({ title: $_("common.delete"), message, confirmLabel: $_("common.delete"), danger: true });
	import type { GameTimePeriodsInput } from "$features/game/types";
	import { Plus, Search } from "@lucide/svelte";
	import { IconButton } from "$/shared/ui";
	import Input from "$/shared/ui/Input.svelte";

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
			name: $_("gameEditor.newTimePeriodName"),
			sortOrder: timePeriods.length,
			startMinute: 0,
			endMinute: 360,
		};
		timePeriods.push(newPeriod);
		selectedIndex = timePeriods.length - 1;
	}

	async function handleDeleteTimePeriod(index: number) {
		if (!await ask($_("gameEditor.deleteTimeConfirm", { values: { name: timePeriods[index].name } }))) return;
		timePeriods.splice(index, 1);
		selectedIndex = timePeriods.length > 0 ? Math.max(0, index - 1) : null;
	}
</script>

<div class="sub-editor-layout">
	<!-- 좌측: 시간대 목록 -->
	<div class="item-list-pane">
		<div class="pane-header">
			<Input bind:value={searchQuery} placeholder={$_("gameEditor.searchTime")} aria-label={$_("gameEditor.searchTime")}>
				{#snippet leading()}<Search />{/snippet}
			</Input>
			<IconButton label={$_("gameEditor.addTimeAria")} size="md" onclick={handleAddTimePeriod}><Plus /></IconButton>
		</div>
		
		<div class="scroll-list">
			{#each filteredPeriods as row (row.tp.id || row.index)}
				{@const tp = row.tp}{@const idx = row.index}
				<div role="none" class="list-item" class:active={selectedIndex === idx} onclick={() => (selectedIndex = idx)}>
					<div class="item-info">
						<span class="title">{tp.name}</span>
						<span class="meta">{$_("gameEditor.minuteRange", { values: { start: tp.startMinute, end: tp.endMinute } })}</span>
					</div>
					<button class="icon-del" onclick={() => handleDeleteTimePeriod(idx)}>×</button>
				</div>
			{:else}
				<div class="empty-text">{$_("gameEditor.noTimePeriods")}</div>
			{/each}
		</div>
	</div>

	<!-- 우측: 상세 편집 폼 -->
	<div class="item-detail-pane">
		{#if activePeriod && selectedIndex !== null}
			<div class="detail-header">
				<h3>{$_("gameEditor.timeDetailHeading")}</h3>
			</div>
			<details class="editor-accordion" open><summary>{$_("gameEditor.timeSettingsAccordion")}</summary><div class="editor-accordion-content"><div class="form-grid">
				<div class="field">
					<label for="time_name">{$_("gameEditor.timeNameLabel")}</label>
					<input id="time_name" type="text" bind:value={activePeriod.name} placeholder={$_("gameEditor.timeNamePlaceholder")} />
				</div>
				<div class="field-row-split">
					<div class="field">
						<label for="time_s_minute">{$_("gameEditor.startMinuteLabel")}</label>
						<input id="time_s_minute" type="number" bind:value={activePeriod.startMinute} min="0" max="1440" />
						<span class="field-hint">{$_("gameEditor.startMinuteHint")}</span>
					</div>
					<div class="field">
						<label for="time_e_minute">{$_("gameEditor.endMinuteLabel")}</label>
						<input id="time_e_minute" type="number" bind:value={activePeriod.endMinute} min="0" max="1440" />
						<span class="field-hint">{$_("gameEditor.endMinuteHint")}</span>
					</div>
				</div>
				<div class="field">
					<label for="time_sort">{$_("gameEditor.sortOrderLabel")}</label>
					<input id="time_sort" type="number" bind:value={activePeriod.sortOrder} />
				</div>
			</div></div></details>
		{:else}
			<div class="no-selection">{$_("gameEditor.selectOrAddTime")}</div>
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
	.icon-del {
		background: transparent;
		border: none;
		color: var(--text-muted);
		font-size: 1rem;
		cursor: pointer;
		padding: 0.3rem;
		border-radius: 4px;
	}
	.icon-del:hover {
		background: var(--danger);
		color: var(--text-on-accent);
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
	.field input {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.6rem;
		color: var(--text);
		font-size: 0.95rem;
	}
	.field input:focus {
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
