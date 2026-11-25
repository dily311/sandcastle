<script lang="ts">
	import { _ } from "svelte-i18n";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	/** 지우기 전에 앱 확인 창으로 물어봄 */
	const ask = (message: string) => confirmAction({ title: $_("common.delete"), message, confirmLabel: $_("common.delete"), danger: true });
	import type {
		GameCharactersInput,
		GameEventsInput,
		GameItemsInput,
		GameSceneInput,
		GameStateDefInput,
	} from "$features/game/types";
	import Input from "$/shared/ui/Input.svelte";
	import { Plus, Search, X } from "@lucide/svelte";
	import IconButton from "$/shared/ui/IconButton.svelte";

	type Props = {
		events: GameEventsInput[];
		items: GameItemsInput[];
		stateDefs: GameStateDefInput[];
		scenes: GameSceneInput[];
		characters: GameCharactersInput[];
		gameId: string;
	};
	let { events = $bindable([]), items, stateDefs, scenes, characters, gameId }: Props = $props();

	let selectedIndex = $state<number | null>(events.length > 0 ? 0 : null);
	let searchQuery = $state("");
	let filteredEvents = $derived(
		events
			.map((ev, index) => ({ ev, index }))
			.filter(({ ev }) =>
				`${ev.name} ${ev.eventType} ${ev.description}`.toLowerCase().includes(searchQuery.trim().toLowerCase()),
			),
	);
	let pendingRelationCharacterId = $state("");
	let activeEvent = $derived(selectedIndex !== null && events[selectedIndex] ? events[selectedIndex] : null);
	let availableRelationCharacters = $derived(
		characters.filter(
			(character) =>
				character.id && !Object.keys(effectSection("relation")).some((key) => key.endsWith(`>${character.id}`)),
		),
	);

	function handleAddEvent() {
		const newEvent: GameEventsInput = {
			id: crypto.randomUUID(),
			gameId: gameId,
			eventType: "custom",
			triggerType: "auto",
			repeatPolicy: "once",
			name: $_("gameEditor.newEventName"),
			description: "",
			requiresNpc: false,
			configJson: {},
			effectJson: {},
			cooldownSeconds: 0,
		};
		events.push(newEvent);
		selectedIndex = events.length - 1;
	}

	async function handleDeleteEvent(index: number) {
		if (!(await ask($_("gameEditor.deleteEventConfirm", { values: { name: events[index].name } })))) return;
		events.splice(index, 1);
		selectedIndex = events.length > 0 ? Math.max(0, index - 1) : null;
	}

	function effectSection(key: string): Record<string, unknown> {
		const value = activeEvent?.effectJson[key];
		return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
	}

	function updateEffectSection(key: string, value: Record<string, unknown>) {
		if (activeEvent) activeEvent.effectJson = { ...activeEvent.effectJson, [key]: value };
	}

	function updateEffectValue(section: string, key: string, value: unknown) {
		updateEffectSection(section, { ...effectSection(section), [key]: value });
	}

	function addVariableEffect(section: "set" | "add") {
		if (!activeEvent) return;
		const effects = effectSection(section);
		const variable = stateDefs.find((item) => !(item.key in effects));
		if (!variable) return;
		let value: unknown = section === "add" ? 1 : variable.defaultValue;
		if (section === "set" && variable.type === "number") value = Number(variable.defaultValue || 0);
		if (section === "set" && variable.type === "boolean") value = variable.defaultValue === "true";
		updateEffectValue(section, variable.key, value);
	}

	function removeEffectValue(section: string, key: string) {
		const next = { ...effectSection(section) };
		delete next[key];
		updateEffectSection(section, next);
	}

	function addItemEffect(section: "give_item" | "take_item") {
		const effects = effectSection(section);
		const item = items.find((candidate) => candidate.id && !(candidate.id in effects));
		if (item?.id) updateEffectValue(section, item.id, 1);
	}

	function updateEffectRoot(key: string, value: string) {
		if (!activeEvent) return;
		const next = { ...activeEvent.effectJson };
		if (value) next[key] = value;
		else delete next[key];
		activeEvent.effectJson = next;
	}

	function updateRelation(characterId: string, axis: string, value: number) {
		const relations = effectSection("relation");
		const pair = {
			...((relations[`player>${characterId}`] as Record<string, unknown> | undefined) ?? {}),
			[axis]: value,
		};
		updateEffectSection("relation", { ...relations, [`player>${characterId}`]: pair });
	}

	function addRelationEffect() {
		const relations = effectSection("relation");
		if (!pendingRelationCharacterId) return;
		updateEffectSection("relation", { ...relations, [`player>${pendingRelationCharacterId}`]: {} });
		pendingRelationCharacterId = "";
	}

	function changeRelationCharacter(oldKey: string, nextId: string) {
		const relations = { ...effectSection("relation") };
		const value = relations[oldKey];
		delete relations[oldKey];
		if (nextId) relations[`player>${nextId}`] = value;
		updateEffectSection("relation", relations);
	}

	function relationCharacterOptions(currentKey: string) {
		const occupiedIds = Object.keys(effectSection("relation"))
			.filter((key) => key !== currentKey)
			.map((key) => key.split(">").at(-1));
		return characters.filter((character) => character.id && !occupiedIds.includes(character.id));
	}

	function removeRelationEffect(key: string) {
		const relations = { ...effectSection("relation") };
		delete relations[key];
		updateEffectSection("relation", relations);
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
			<Input bind:value={searchQuery} placeholder={$_("gameEditor.searchEvents")} aria-label={$_("gameEditor.searchEvents")}>
				{#snippet leading()}<Search />{/snippet}
			</Input>
			<IconButton label={$_("gameEditor.addEventAria")} size="md" onclick={handleAddEvent}><Plus /></IconButton>
		</div>
		<div class="scroll-list">
			{#each filteredEvents as row (row.ev.id)}
				{@const ev = row.ev}
				{@const idx = row.index}
				<div
					role="none"
					class="list-item"
					class:active={selectedIndex === idx}
					onclick={() => (selectedIndex = idx)}
				>
					<div class="item-info">
						<span class="title">{ev.name}</span>
						<span class="meta">{$_("gameEditor.eventTypeMeta", { values: { type: ev.eventType } })}</span>
					</div>
					<IconButton class="icon-del" size="sm" label={$_("gameEditor.deleteEventAria")} onclick={() => handleDeleteEvent(idx)}
						><X />
					</IconButton>
				</div>
			{:else}
				<div class="empty-text">{$_("gameEditor.noEvents")}</div>
			{/each}
		</div>
	</div>

	<!-- 우측: 상세 편집 폼 -->
	<div class="item-detail-pane">
		{#if activeEvent && selectedIndex !== null}
			<div class="detail-header">
				<h3>{$_("gameEditor.eventDetailHeading")}</h3>
			</div>
			<div class="form-grid">
				<div class="field-row-split">
					<div class="field">
						<label for="event_name">{$_("gameEditor.eventNameLabel")}</label>
						<input id="event_name" type="text" bind:value={activeEvent.name} />
					</div>
					<div class="field">
						<label for="event_type">{$_("gameEditor.eventTypeLabel")}</label>
						<input
							id="event_type"
							type="text"
							bind:value={activeEvent.eventType}
							placeholder={$_("gameEditor.eventTypePlaceholder")}
						/>
					</div>
				</div>

				<div class="field">
					<label for="event_des">{$_("gameEditor.sceneDescLabel")}</label>
					<textarea id="event_des" bind:value={activeEvent.description} rows="2" placeholder={$_("gameEditor.eventDescPlaceholder")}
					></textarea>
				</div>

				<div class="field-row-split">
					<div class="field">
						<label for="event_seconds">{$_("gameEditor.cooldownLabel")}</label>
						<input id="event_seconds" type="number" bind:value={activeEvent.cooldownSeconds} min="0" />
					</div>
					<div class="field checkbox-field">
						<label class="checkbox-label">
							<input type="checkbox" bind:checked={activeEvent.requiresNpc} />
							<span>{$_("gameEditor.requiresNpcLabel")}</span>
						</label>
					</div>
				</div>

				{#if activeEvent.triggerType === "manual"}
					<div class="field checkbox-field">
						<label class="checkbox-label">
							<input
								type="checkbox"
								checked={(activeEvent.configJson as Record<string, unknown>)?.planUsable === true}
								onchange={(e) => (activeEvent!.configJson = { ...(activeEvent!.configJson as Record<string, unknown>), planUsable: e.currentTarget.checked })} />
							<span>{$_("gameEditor.planUsableLabel")}</span>
						</label>
					</div>
				{/if}

				<div class="field">
					<label for="event_setting">{$_("gameEditor.advancedConfigLabel")}</label>
					<textarea
						id="event_setting"
						class="code-font"
						rows="4"
						value={toJsonString(activeEvent.configJson)}
						oninput={(e) => fromJsonString(e.currentTarget.value, "configJson")}></textarea>
				</div>
				<details class="editor-accordion effect-editor" open>
					<summary>{$_("gameEditor.eventEffectsAccordion")}</summary>
					<div class="editor-accordion-content">
						<div class="effect-heading">
							<div><p>{$_("gameEditor.eventEffectsDesc")}</p></div>
						</div>
						<div class="effect-section">
							<div class="effect-section-title">
								<strong>{$_("gameEditor.varChangeHeading")}</strong>
								<div>
									<button class="small-action" onclick={() => addVariableEffect("set")}
										>{$_("gameEditor.setValueBtn")}</button
									><button class="small-action" onclick={() => addVariableEffect("add")}
										>{$_("gameEditor.addNumberBtn")}</button
									>
								</div>
							</div>
							{#each Object.entries(effectSection("set")) as [key, value] (`set:${key}`)}
								<div class="effect-row">
									<span class="effect-kind">{$_("gameEditor.kindSet")}</span><select
										value={key}
										onchange={(e) => {
											const next = e.currentTarget.value;
											const current = effectSection("set")[key];
											removeEffectValue("set", key);
											updateEffectValue("set", next, current);
										}}
										><option value={key}>{key}</option
										>{#each stateDefs as variable (variable.id)}<option value={variable.key}
												>{variable.key}</option
											>{/each}</select
									>{#if stateDefs.find((variable) => variable.key === key)?.type === "boolean"}<select
											value={String(value)}
											onchange={(e) =>
												updateEffectValue("set", key, e.currentTarget.value === "true")}
											><option value="true">{$_("gameEditor.boolTrue")}</option><option value="false">{$_("gameEditor.boolFalse")}</option></select
										>{:else}<input
											aria-label={$_("gameEditor.setValueAria")}
											type={stateDefs.find((variable) => variable.key === key)?.type === "number"
												? "number"
												: "text"}
											value={String(value ?? "")}
											oninput={(e) =>
												updateEffectValue(
													"set",
													key,
													stateDefs.find((variable) => variable.key === key)?.type ===
														"number"
														? Number(e.currentTarget.value)
														: e.currentTarget.value,
												)}
										/>{/if}<button
										class="row-remove"
										aria-label={$_("gameEditor.deleteVarEffectAria")}
										onclick={() => removeEffectValue("set", key)}>×</button
									>
								</div>
							{/each}
							{#each Object.entries(effectSection("add")) as [key, value] (`add:${key}`)}
								<div class="effect-row">
									<span class="effect-kind">{$_("gameEditor.kindAdd")}</span><select
										value={key}
										onchange={(e) => {
											const next = e.currentTarget.value;
											const current = effectSection("add")[key];
											removeEffectValue("add", key);
											updateEffectValue("add", next, current);
										}}
										><option value={key}>{key}</option
										>{#each stateDefs.filter((variable) => variable.type === "number") as variable (variable.id)}<option
												value={variable.key}>{variable.key}</option
											>{/each}</select
									><input
										aria-label={$_("gameEditor.addValueAria")}
										type="number"
										value={Number(value)}
										oninput={(e) => updateEffectValue("add", key, Number(e.currentTarget.value))}
									/><button
										class="row-remove"
										aria-label={$_("gameEditor.deleteVarEffectAria")}
										onclick={() => removeEffectValue("add", key)}>×</button
									>
								</div>
							{/each}
							{#if !Object.keys(effectSection("set")).length && !Object.keys(effectSection("add")).length}<p
									class="effect-empty"
								>
									{$_("gameEditor.noVarEffects")}
								</p>{/if}
						</div>
						<div class="effect-section">
							<div class="effect-section-title">
								<strong>{$_("gameEditor.itemEffectHeading")}</strong>
								<div>
									<button class="small-action" onclick={() => addItemEffect("give_item")}
										>{$_("gameEditor.giveItemBtn")}</button
									><button class="small-action" onclick={() => addItemEffect("take_item")}
										>{$_("gameEditor.takeItemBtn")}</button
									>
								</div>
							</div>
							{#each ["give_item", "take_item"] as section}
								{#each Object.entries(effectSection(section)) as [itemId, quantity] (`${section}:${itemId}`)}
									<div class="effect-row">
										<span class="effect-kind">{section === "give_item" ? $_("gameEditor.kindGive") : $_("gameEditor.kindTake")}</span
										><select
											value={itemId}
											onchange={(e) => {
												const next = e.currentTarget.value;
												const current = effectSection(section)[itemId];
												removeEffectValue(section, itemId);
												updateEffectValue(section, next, current);
											}}
											><option value={itemId}
												>{items.find((item) => item.id === itemId)?.name ?? itemId}</option
											>{#each items as item (item.id)}<option value={item.id ?? ""}
													>{item.name}</option
												>{/each}</select
										><input
											aria-label={$_("gameEditor.quantityAria")}
											type="number"
											min="1"
											value={Number(quantity)}
											oninput={(e) =>
												updateEffectValue(
													section,
													itemId,
													Math.max(1, Number(e.currentTarget.value)),
												)}
										/><button
											class="row-remove"
											aria-label={$_("gameEditor.deleteItemEffectAria")}
											onclick={() => removeEffectValue(section, itemId)}>×</button
										>
									</div>
								{/each}
							{/each}
						</div>
						<div class="effect-section">
							<div class="effect-section-title">
								<strong>{$_("gameEditor.relationHeading")}</strong>
								<div class="relation-add">
									<select
										aria-label={$_("gameEditor.selectRelationCharAria")}
										bind:value={pendingRelationCharacterId}
										><option value="">{$_("gameEditor.selectCharOption")}</option
										>{#each availableRelationCharacters as character (character.id)}<option
												value={character.id ?? ""}>{character.name}</option
											>{/each}</select
									><button
										class="small-action"
										onclick={addRelationEffect}
										disabled={!pendingRelationCharacterId}>{$_("common.add")}</button
									>
								</div>
							</div>
							{#each Object.entries(effectSection("relation")) as [key, rawValues] (key)}
								{@const relationValues = rawValues as Record<string, unknown>}
								{@const characterId = key.split(">").at(-1) ?? ""}
								<div class="relation-row">
									<select
										aria-label={$_("gameEditor.relationCharAria")}
										value={characterId}
										onchange={(e) => changeRelationCharacter(key, e.currentTarget.value)}
										>{#each relationCharacterOptions(key) as character (character.id)}<option
												value={character.id ?? ""}>{character.name}</option
											>{/each}</select
									>{#each ["affection", "trust", "tension"] as axis}<label
											>{axis}<input
												type="number"
												value={Number(relationValues[axis] ?? 0)}
												oninput={(e) =>
													updateRelation(characterId, axis, Number(e.currentTarget.value))}
											/></label
										>{/each}<button
										class="row-remove"
										aria-label={$_("gameEditor.deleteRelationAria")}
										onclick={() => removeRelationEffect(key)}>×</button
									>
								</div>
							{/each}
							{#if !Object.keys(effectSection("relation")).length}<p class="effect-empty">
									{$_("gameEditor.noRelationEffects")}
								</p>{/if}
						</div>
						<div class="effect-section effect-routing">
							<label
								>{$_("gameEditor.nextSceneLabel")}<select
									value={String(activeEvent.effectJson.enter_scene ?? "")}
									onchange={(e) => updateEffectRoot("enter_scene", e.currentTarget.value)}
									><option value="">{$_("gameEditor.noMoveOption")}</option>{#each scenes as scene (scene.id)}<option
											value={scene.id}>{scene.sceneName}</option
										>{/each}</select
								></label
							>
							<label
								>{$_("gameEditor.fireEventLabel")}<select
									value={String(activeEvent.effectJson.fire_event ?? "")}
									onchange={(e) => updateEffectRoot("fire_event", e.currentTarget.value)}
									><option value="">{$_("gameEditor.noLinkOption")}</option
									>{#each events.filter((event) => event.id !== activeEvent.id) as event (event.id)}<option
											value={event.id}>{event.name}</option
										>{/each}</select
								></label
							>
						</div>
					</div>
				</details>
			</div>
		{:else}
			<div class="no-selection">{$_("gameEditor.selectOrAddEvent")}</div>
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
	.field textarea {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.6rem;
		color: var(--text);
		font-size: 0.95rem;
	}
	.field input:focus,
	.field textarea:focus {
		outline: none;
		border-color: var(--accent);
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
		color: var(--text);
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
	.effect-editor {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-top: 0.3rem;
	}
	.effect-heading h4 {
		margin: 0;
		color: var(--text);
		font-size: 1rem;
	}
	.effect-heading p {
		margin: 0.3rem 0 0;
		color: var(--text-muted);
		font-size: 0.78rem;
	}
	.effect-section {
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
		padding: 0.85rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-2);
	}
	.effect-section-title {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.6rem;
	}
	.effect-section-title strong,
	.effect-section > strong {
		color: var(--text);
		font-size: 0.82rem;
	}
	.effect-section-title > div {
		display: flex;
		gap: 0.35rem;
	}
	.small-action {
		padding: 0.35rem 0.55rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		background: var(--surface);
		color: var(--text-muted);
		font: inherit;
		font-size: 0.72rem;
		cursor: pointer;
	}
	.small-action:hover {
		color: var(--accent);
		border-color: var(--accent);
	}
	.effect-row {
		display: grid;
		grid-template-columns: 68px minmax(120px, 1fr) minmax(100px, 0.7fr) 28px;
		gap: 0.45rem;
		align-items: center;
	}
	.effect-row input,
	.effect-row select,
	.effect-routing input,
	.effect-routing select,
	.relation-row input,
	.relation-add select {
		min-width: 0;
		width: 100%;
		box-sizing: border-box;
		padding: 0.52rem 0.6rem;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.8rem;
	}
	.effect-kind {
		color: var(--text-muted);
		font-size: 0.72rem;
	}
	.row-remove {
		border: 0;
		background: transparent;
		color: var(--text-muted);
		font-size: 1.15rem;
		cursor: pointer;
	}
	.row-remove:hover {
		color: var(--danger);
	}
	.effect-empty {
		margin: 0;
		padding: 0.5rem 0;
		color: var(--text-faint);
		font-size: 0.76rem;
	}
	.effect-routing {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.65rem;
	}
	.effect-routing label,
	.relation-row label {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		color: var(--text-muted);
		font-size: 0.72rem;
		font-weight: 600;
	}
	.relation-add {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}
	.relation-row {
		display: grid;
		grid-template-columns: minmax(130px, 1fr) repeat(3, minmax(75px, 0.6fr)) 24px;
		align-items: end;
		gap: 0.55rem;
		padding: 0.45rem 0;
		border-top: 1px solid var(--border);
	}
	.relation-row > select {
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
		padding: 0.52rem 0.6rem;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.8rem;
	}
	@media (max-width: 700px) {
		.effect-row {
			grid-template-columns: 60px minmax(100px, 1fr) minmax(80px, 0.8fr) 24px;
		}
		.effect-routing {
			grid-template-columns: 1fr;
		}
		.relation-row {
			grid-template-columns: minmax(95px, 1fr) repeat(3, minmax(55px, 0.6fr)) 22px;
			gap: 0.3rem;
		}
	}
</style>
