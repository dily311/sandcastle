<script lang="ts">
	import type { GameCharactersInput, GameEventsInput, GameItemsInput, GameSceneInput, GameStateDefInput } from "$/lib/type/game";

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
	let filteredEvents = $derived(events.map((ev, index) => ({ ev, index })).filter(({ ev }) => `${ev.name} ${ev.eventType} ${ev.description}`.toLowerCase().includes(searchQuery.trim().toLowerCase())));
	let pendingRelationCharacterId = $state("");
	let activeEvent = $derived(selectedIndex !== null && events[selectedIndex] ? events[selectedIndex] : null);
	let availableRelationCharacters = $derived(characters.filter((character) => character.id && !Object.keys(effectSection("relation")).some((key) => key.endsWith(`>${character.id}`))));

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

	function effectSection(key: string): Record<string, unknown> {
		const value = activeEvent?.effectJson[key];
		return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
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
		const pair = { ...((relations[`player>${characterId}`] as Record<string, unknown> | undefined) ?? {}), [axis]: value };
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
		<label class="list-search"><span aria-hidden="true">⌕</span><input aria-label="이벤트 검색" placeholder="이벤트 검색" bind:value={searchQuery} /></label>
		<div class="pane-header">
			<h4>이벤트 목록 ({events.length})</h4>
			<button class="add-btn" onclick={handleAddEvent}>+ 이벤트 추가</button>
		</div>
		<div class="scroll-list">
			{#each filteredEvents as row (row.ev.id)}
				{@const ev = row.ev}{@const idx = row.index}
				<div
					role="none"
					class="list-item"
					class:active={selectedIndex === idx}
					onclick={() => (selectedIndex = idx)}
				>
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
			</div>
			<div class="form-grid">
				<div class="field-row-split">
					<div class="field">
						<label for="event_name">이벤트 이름 (Name)</label>
						<input id="event_name" type="text" bind:value={activeEvent.name} />
					</div>
					<div class="field">
						<label for="event_type">이벤트 유형 (Event Type)</label>
						<input
							id="event_type"
							type="text"
							bind:value={activeEvent.eventType}
							placeholder="예: combat, dialog, script"
						/>
					</div>
				</div>

				<div class="field">
					<label for="event_des">설명 (Description)</label>
					<textarea id="event_des" bind:value={activeEvent.description} rows="2" placeholder="이벤트 설명..."
					></textarea>
				</div>

				<div class="field-row-split">
					<div class="field">
						<label for="event_seconds">쿨다운 시간 (Seconds)</label>
						<input id="event_seconds" type="number" bind:value={activeEvent.cooldownSeconds} min="0" />
					</div>
					<div class="field checkbox-field">
						<label class="checkbox-label">
							<input type="checkbox" bind:checked={activeEvent.requiresNpc} />
							<span>NPC 필요 여부 (Requires NPC)</span>
						</label>
					</div>
				</div>

				<div class="field">
					<label for="event_setting">고급 설정 JSON</label>
						<textarea
							id="event_setting"
							class="code-font"
							rows="4"
							value={toJsonString(activeEvent.configJson)}
							oninput={(e) => fromJsonString(e.currentTarget.value, "configJson")}></textarea>
				</div>
				<details class="editor-accordion effect-editor" open><summary>이벤트 효과</summary><div class="editor-accordion-content">
					<div class="effect-heading"><div><p>정해진 항목을 입력하면 저장 시 엔진 효과 형식으로 구성됩니다.</p></div></div>
					<div class="effect-section">
						<div class="effect-section-title"><strong>변수 변경</strong><div><button class="small-action" onclick={() => addVariableEffect("set")}>값 지정</button><button class="small-action" onclick={() => addVariableEffect("add")}>숫자 더하기</button></div></div>
						{#each Object.entries(effectSection("set")) as [key, value] (`set:${key}`)}
							<div class="effect-row"><span class="effect-kind">지정</span><select value={key} onchange={(e) => { const next = e.currentTarget.value; const current = effectSection("set")[key]; removeEffectValue("set", key); updateEffectValue("set", next, current); }}><option value={key}>{key}</option>{#each stateDefs as variable (variable.id)}<option value={variable.key}>{variable.key}</option>{/each}</select>{#if stateDefs.find((variable) => variable.key === key)?.type === "boolean"}<select value={String(value)} onchange={(e) => updateEffectValue("set", key, e.currentTarget.value === "true")}><option value="true">참</option><option value="false">거짓</option></select>{:else}<input aria-label="지정할 값" type={stateDefs.find((variable) => variable.key === key)?.type === "number" ? "number" : "text"} value={String(value ?? "")} oninput={(e) => updateEffectValue("set", key, stateDefs.find((variable) => variable.key === key)?.type === "number" ? Number(e.currentTarget.value) : e.currentTarget.value)} />{/if}<button class="row-remove" aria-label="변수 효과 삭제" onclick={() => removeEffectValue("set", key)}>×</button></div>
						{/each}
						{#each Object.entries(effectSection("add")) as [key, value] (`add:${key}`)}
							<div class="effect-row"><span class="effect-kind">더하기</span><select value={key} onchange={(e) => { const next = e.currentTarget.value; const current = effectSection("add")[key]; removeEffectValue("add", key); updateEffectValue("add", next, current); }}><option value={key}>{key}</option>{#each stateDefs.filter((variable) => variable.type === "number") as variable (variable.id)}<option value={variable.key}>{variable.key}</option>{/each}</select><input aria-label="더할 값" type="number" value={Number(value)} oninput={(e) => updateEffectValue("add", key, Number(e.currentTarget.value))} /><button class="row-remove" aria-label="변수 효과 삭제" onclick={() => removeEffectValue("add", key)}>×</button></div>
						{/each}
						{#if !Object.keys(effectSection("set")).length && !Object.keys(effectSection("add")).length}<p class="effect-empty">아직 변수 효과가 없습니다.</p>{/if}
					</div>
					<div class="effect-section">
						<div class="effect-section-title"><strong>아이템 지급 / 회수</strong><div><button class="small-action" onclick={() => addItemEffect("give_item")}>아이템 지급</button><button class="small-action" onclick={() => addItemEffect("take_item")}>아이템 회수</button></div></div>
						{#each ["give_item", "take_item"] as section}
							{#each Object.entries(effectSection(section)) as [itemId, quantity] (`${section}:${itemId}`)}
								<div class="effect-row"><span class="effect-kind">{section === "give_item" ? "지급" : "회수"}</span><select value={itemId} onchange={(e) => { const next = e.currentTarget.value; const current = effectSection(section)[itemId]; removeEffectValue(section, itemId); updateEffectValue(section, next, current); }}><option value={itemId}>{items.find((item) => item.id === itemId)?.name ?? itemId}</option>{#each items as item (item.id)}<option value={item.id ?? ""}>{item.name}</option>{/each}</select><input aria-label="수량" type="number" min="1" value={Number(quantity)} oninput={(e) => updateEffectValue(section, itemId, Math.max(1, Number(e.currentTarget.value)))} /><button class="row-remove" aria-label="아이템 효과 삭제" onclick={() => removeEffectValue(section, itemId)}>×</button></div>
							{/each}
						{/each}
					</div>
					<div class="effect-section">
						<div class="effect-section-title"><strong>캐릭터 관계 변화</strong><div class="relation-add"><select aria-label="관계를 변경할 캐릭터 선택" bind:value={pendingRelationCharacterId}><option value="">캐릭터 선택</option>{#each availableRelationCharacters as character (character.id)}<option value={character.id ?? ""}>{character.name}</option>{/each}</select><button class="small-action" onclick={addRelationEffect} disabled={!pendingRelationCharacterId}>추가</button></div></div>
						{#each Object.entries(effectSection("relation")) as [key, rawValues] (key)}
							{@const relationValues = rawValues as Record<string, unknown>}
							{@const characterId = key.split(">").at(-1) ?? ""}
							<div class="relation-row"><select aria-label="관계 변경 캐릭터" value={characterId} onchange={(e) => changeRelationCharacter(key, e.currentTarget.value)}>{#each relationCharacterOptions(key) as character (character.id)}<option value={character.id ?? ""}>{character.name}</option>{/each}</select>{#each ["affection", "trust", "tension"] as axis}<label>{axis}<input type="number" value={Number(relationValues[axis] ?? 0)} oninput={(e) => updateRelation(characterId, axis, Number(e.currentTarget.value))} /></label>{/each}<button class="row-remove" aria-label="관계 효과 삭제" onclick={() => removeRelationEffect(key)}>×</button></div>
						{/each}
						{#if !Object.keys(effectSection("relation")).length}<p class="effect-empty">관계 변화가 필요하면 캐릭터를 추가하세요.</p>{/if}
					</div>
					<div class="effect-section effect-routing">
						<label>이동할 씬<select value={String(activeEvent.effectJson.enter_scene ?? "")} onchange={(e) => updateEffectRoot("enter_scene", e.currentTarget.value)}><option value="">이동 없음</option>{#each scenes as scene (scene.id)}<option value={scene.id}>{scene.sceneName}</option>{/each}</select></label>
						<label>실행할 이벤트<select value={String(activeEvent.effectJson.fire_event ?? "")} onchange={(e) => updateEffectRoot("fire_event", e.currentTarget.value)}><option value="">연결 없음</option>{#each events.filter((event) => event.id !== activeEvent.id) as event (event.id)}<option value={event.id}>{event.name}</option>{/each}</select></label>
					</div>
				</div></details>
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
	.effect-editor { display:flex; flex-direction:column; gap:.75rem; margin-top:.3rem; }
	.effect-heading h4 { margin:0; color:var(--text-color); font-size:1rem; }
	.effect-heading p { margin:.3rem 0 0; color:var(--text-muted); font-size:.78rem; }
	.effect-section { display:flex; flex-direction:column; gap:.55rem; padding:.85rem; border:1px solid var(--border-color); border-radius:var(--radius-md); background:var(--background-secondary); }
	.effect-section-title { display:flex; align-items:center; justify-content:space-between; gap:.6rem; }
	.effect-section-title strong,.effect-section>strong { color:var(--text-color); font-size:.82rem; }
	.effect-section-title>div { display:flex; gap:.35rem; }
	.small-action { padding:.35rem .55rem; border:1px solid var(--border-color); border-radius:var(--radius-sm); background:var(--card-bg); color:var(--text-muted); font:inherit; font-size:.72rem; cursor:pointer; }
	.small-action:hover { color:var(--accent-color); border-color:var(--accent-color); }
	.effect-row { display:grid; grid-template-columns:68px minmax(120px,1fr) minmax(100px,.7fr) 28px; gap:.45rem; align-items:center; }
	.effect-row input,.effect-row select,.effect-routing input,.effect-routing select,.relation-row input,.relation-add select { min-width:0; width:100%; box-sizing:border-box; padding:.52rem .6rem; border:1px solid var(--input-border-color); border-radius:var(--radius-sm); background:var(--input-bg); color:var(--input-color); font:inherit; font-size:.8rem; }
	.effect-kind { color:var(--text-muted); font-size:.72rem; }
	.row-remove { border:0; background:transparent; color:var(--text-muted); font-size:1.15rem; cursor:pointer; }
	.row-remove:hover { color:var(--color-red); }
	.effect-empty { margin:0; padding:.5rem 0; color:var(--text-faint); font-size:.76rem; }
	.effect-routing { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:.65rem; }
	.effect-routing label,.relation-row label { display:flex; flex-direction:column; gap:.3rem; color:var(--text-muted); font-size:.72rem; font-weight:600; }
	.relation-add { display:flex; align-items:center; gap:.35rem; }
	.relation-row { display:grid; grid-template-columns:minmax(130px,1fr) repeat(3,minmax(75px,.6fr)) 24px; align-items:end; gap:.55rem; padding:.45rem 0; border-top:1px solid var(--border-color); }
	.relation-row>select { width:100%; min-width:0; box-sizing:border-box; padding:.52rem .6rem; border:1px solid var(--input-border-color); border-radius:var(--radius-sm); background:var(--input-bg); color:var(--input-color); font:inherit; font-size:.8rem; }
	@media (max-width:700px) { .effect-row { grid-template-columns:60px minmax(100px,1fr) minmax(80px,.8fr) 24px; } .effect-routing { grid-template-columns:1fr; } .relation-row { grid-template-columns:minmax(95px,1fr) repeat(3,minmax(55px,.6fr)) 22px; gap:.3rem; } }
</style>
