<script lang="ts">
	import { _ } from "svelte-i18n";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	/** 지우기 전에 앱 확인 창으로 물어봄 */
	const ask = (message: string) => confirmAction({ title: $_("common.delete"), message, confirmLabel: $_("common.delete"), danger: true });
	import type { GameMapsInput, GameWeatherTransitionsInput, GameWeatherTypesInput } from "$features/game/types";
	import IconButton from "$/shared/ui/IconButton.svelte";
	import Input from "$/shared/ui/Input.svelte";
	import { Plus, Search, X } from "@lucide/svelte";
	type Props = {
		weatherTypes: GameWeatherTypesInput[];
		weatherTransitions: GameWeatherTransitionsInput[];
		maps: GameMapsInput[];
		gameId: string;
	};
	let { weatherTypes = $bindable([]), weatherTransitions = $bindable([]), maps = [], gameId }: Props = $props();
	let selectedIndex = $state<number | null>(weatherTypes.length ? 0 : null);
	let searchQuery = $state("");
	let filteredWeather = $derived(
		weatherTypes
			.map((weather, index) => ({ weather, index }))
			.filter(({ weather }) => weather.name.toLowerCase().includes(searchQuery.trim().toLowerCase())),
	);
	let selected = $derived(selectedIndex !== null ? (weatherTypes[selectedIndex] ?? null) : null);
	let selectedTransitions = $derived(
		selected
			? weatherTransitions
					.map((transition, index) => ({ transition, index }))
					.filter(({ transition }) => transition.fromWeatherId === selected.id)
			: [],
	);
	function addWeather() {
		weatherTypes.push({
			id: crypto.randomUUID(),
			gameId,
			name: $_("gameEditor.newWeatherName"),
			encounterRateModifier: 1,
			visibilityModifier: 1,
		});
		selectedIndex = weatherTypes.length - 1;
	}
	async function removeWeather(i: number) {
		if (!(await ask($_("gameEditor.deleteWeatherConfirm", { values: { name: weatherTypes[i].name } })))) return;
		const removedId = weatherTypes[i].id;
		weatherTypes.splice(i, 1);
		for (let ruleIndex = weatherTransitions.length - 1; ruleIndex >= 0; ruleIndex--)
			if (
				weatherTransitions[ruleIndex].fromWeatherId === removedId ||
				weatherTransitions[ruleIndex].toWeatherId === removedId
			)
				weatherTransitions.splice(ruleIndex, 1);
		selectedIndex = weatherTypes.length ? Math.max(0, i - 1) : null;
	}
	function addTransition() {
		if (!selected || !maps.length) {
			toasts.info($_("gameEditor.needMapForTransition"));
			return;
		}
		weatherTransitions.push({
			fromWeatherId: selected.id ?? "",
			toWeatherId: weatherTypes.find((weather) => weather.id !== selected.id)?.id ?? selected.id ?? "",
			mapId: maps[0].id ?? "",
			probability: 0.5,
		});
	}
	async function removeTransition(i: number) {
		if (await ask($_("gameEditor.deleteTransitionConfirm"))) weatherTransitions.splice(i, 1);
	}
</script>

<div class="sub-editor-layout">
	<aside class="item-list-pane">
		<div class="pane-header">
			<Input bind:value={searchQuery} placeholder={$_("gameEditor.searchWeather")} aria-label={$_("gameEditor.searchWeather")}>
				{#snippet leading()}
					<Search />
				{/snippet}
			</Input>
			<IconButton label={$_("gameEditor.addWeatherAria")} size="md" onclick={addWeather}>
				<Plus />
			</IconButton>
		</div>
		<div class="scroll-list">
			{#each filteredWeather as row (row.weather.id || row.index)}
				{@const ev = row.weather}
				{@const idx = row.index}
				<div
					role="none"
					class="list-item"
					class:active={selectedIndex === idx}
					onclick={() => (selectedIndex = idx)}
				>
					<div class="item-info">
						<span class="title">{ev.name}</span>
						<span class="meta">{$_("gameEditor.encounterMeta", { values: { val: ev.encounterRateModifier } })}</span>
					</div>
					<IconButton class="icon-del" size="sm" label={$_("gameEditor.deleteWeatherAria")} onclick={() => removeWeather(idx)}><X /></IconButton>
				</div>
			{:else}
				<p class="empty">{$_("gameEditor.noWeather")}</p>
			{/each}
		</div>
	</aside>
	<section class="item-detail-pane">
		{#if selected}
			<details class="editor-accordion" open>
				<summary> {$_("gameEditor.weatherSettingsAccordion", { values: { name: selected.name } })}</summary>
				<div class="editor-accordion-content">
					<div class="fields">
						<label>{$_("gameEditor.weatherNameLabel")}<input bind:value={selected.name} /> </label>
						<label
							>{$_("gameEditor.encounterModLabel")}<input type="number" step="0.1" bind:value={selected.encounterRateModifier} />
						</label>
						<label
							>{$_("gameEditor.visibilityModLabel")}<input type="number" step="0.1" bind:value={selected.visibilityModifier} />
						</label>
					</div>
				</div>
			</details>
			<details class="editor-accordion" open>
				<summary> {$_("gameEditor.transitionsAccordion", { values: { name: selected.name, count: selectedTransitions.length } })}</summary>
				<div class="editor-accordion-content transition-content">
					<button class="add" onclick={addTransition}>{$_("gameEditor.addTransitionBtn")}</button>
					{#each selectedTransitions as row (`${row.transition.fromWeatherId}-${row.index}`)}{@const transition =
							row.transition}
						<div class="transition-card">
							<span class="from-weather">
								{selected.name}
							</span>
							<span class="arrow">→</span>
							<label
								>{$_("gameEditor.toWeatherLabel")}<select bind:value={transition.toWeatherId}>
									{#each weatherTypes as weather (weather.id)}
										<option value={weather.id}>
											{weather.name}
										</option>
									{/each}
								</select>
							</label>
							<label
								>{$_("gameEditor.targetMapLabel")}<select bind:value={transition.mapId}>
									{#each maps as map (map.id)}
										<option value={map.id}>
											{map.name}
										</option>
									{/each}
								</select>
							</label>
							<label
								>{$_("gameEditor.probabilityLabel")}<input
									type="number"
									min="0"
									max="1"
									step="0.05"
									bind:value={transition.probability}
								/>
							</label>
							<button
								class="remove"
								aria-label={$_("gameEditor.deleteTransitionAria")}
								onclick={() => removeTransition(row.index)}>×</button
							>
						</div>
					{:else}
						<p class="empty">{$_("gameEditor.noTransitions")}</p>
					{/each}
				</div>
			</details>
		{:else}
			<div class="empty">{$_("gameEditor.selectOrAddWeather")}</div>
		{/if}
	</section>
</div>

<style>
	.empty {
		color: var(--text-muted);
		font-size: 0.78rem;
	}
	.remove {
		border: 0;
		background: none;
		color: var(--text-muted);
		font-size: 1.25rem;
		padding: 0.45rem 0.7rem;
		cursor: pointer;
	}
	.remove:hover {
		color: var(--accent);
	}
	.add {
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		padding: 0.5rem 0.7rem;
		background: var(--surface-2);
		color: var(--text);
		cursor: pointer;
		font: inherit;
		font-size: 0.78rem;
	}
	.add:hover {
		background: var(--surface-hover);
	}
	.fields {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 0.75rem;
	}
	.fields label,
	.transition-card label {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		font-size: 0.78rem;
		color: var(--text-muted);
	}
	.fields input,
	.transition-card select,
	.transition-card input {
		width: 100%;
		box-sizing: border-box;
		padding: 0.55rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--bg);
		color: var(--text);
		font: inherit;
	}
	.transition-content {
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
	}
	.transition-card {
		display: grid;
		grid-template-columns: 1fr auto 1fr 1.2fr 0.8fr auto;
		gap: 0.6rem;
		align-items: end;
		padding: 0.7rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-2);
	}
	.from-weather {
		align-self: center;
		color: var(--text);
		font-weight: 650;
	}
	.arrow {
		padding-bottom: 0.5rem;
		color: var(--text-muted);
	}
	@media (max-width: 760px) {
		.fields {
			grid-template-columns: 1fr;
		}
		.transition-card {
			grid-template-columns: 1fr 1fr;
		}
		.transition-card .arrow {
			display: none;
		}
	}
</style>
