<script lang="ts">
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	/** 지우기 전에 앱 확인 창으로 물어봄 */
	const ask = (message: string) => confirmAction({ title: "삭제", message, confirmLabel: "삭제", danger: true });
	import type { GameMapsInput, GameWeatherTransitionsInput, GameWeatherTypesInput } from "$/lib/type/game";
	type Props = { weatherTypes: GameWeatherTypesInput[]; weatherTransitions: GameWeatherTransitionsInput[]; maps: GameMapsInput[]; gameId: string };
	let { weatherTypes = $bindable([]), weatherTransitions = $bindable([]), maps = [], gameId }: Props = $props();
	let selectedIndex = $state<number | null>(weatherTypes.length ? 0 : null);
	let searchQuery = $state("");
	let filteredWeather = $derived(weatherTypes.map((weather, index) => ({ weather, index })).filter(({ weather }) => weather.name.toLowerCase().includes(searchQuery.trim().toLowerCase())));
	let selected = $derived(selectedIndex !== null ? weatherTypes[selectedIndex] ?? null : null);
	let selectedTransitions = $derived(selected ? weatherTransitions.map((transition, index) => ({ transition, index })).filter(({ transition }) => transition.fromWeatherId === selected.id) : []);
	function addWeather() { weatherTypes.push({ id: crypto.randomUUID(), gameId, name: "새로운 날씨", encounterRateModifier: 1, visibilityModifier: 1 }); selectedIndex = weatherTypes.length - 1; }
	async function removeWeather(i: number) { if (!await ask(`"${weatherTypes[i].name}" 날씨를 삭제하시겠습니까?`)) return; const removedId = weatherTypes[i].id; weatherTypes.splice(i, 1); for (let ruleIndex = weatherTransitions.length - 1; ruleIndex >= 0; ruleIndex--) if (weatherTransitions[ruleIndex].fromWeatherId === removedId || weatherTransitions[ruleIndex].toWeatherId === removedId) weatherTransitions.splice(ruleIndex, 1); selectedIndex = weatherTypes.length ? Math.max(0, i - 1) : null; }
	function addTransition() { if (!selected || !maps.length) { toasts.info("전환 규칙을 추가하려면 맵을 먼저 등록해주세요."); return; } weatherTransitions.push({ fromWeatherId: selected.id ?? "", toWeatherId: weatherTypes.find((weather) => weather.id !== selected.id)?.id ?? selected.id ?? "", mapId: maps[0].id ?? "", probability: 0.5 }); }
	async function removeTransition(i: number) { if (await ask("이 날씨 전환 규칙을 삭제하시겠습니까?")) weatherTransitions.splice(i, 1); }
</script>

<div class="weather-layout">
	<aside class="weather-list"><label class="list-search"><span aria-hidden="true">⌕</span><input aria-label="날씨 검색" placeholder="날씨 검색" bind:value={searchQuery} /></label>
		<header><h3>날씨 목록 ({weatherTypes.length})</h3><button class="add" onclick={addWeather}>+ 날씨 추가</button></header>
		{#each filteredWeather as row (row.weather.id || row.index)}{@const weather = row.weather}{@const i = row.index}<div class="weather-item" class:active={selectedIndex === i}><button class="pick" onclick={() => selectedIndex = i}><strong>{weather.name}</strong><small>조우 보정 {weather.encounterRateModifier}x</small></button><button class="remove" aria-label="날씨 삭제" onclick={() => removeWeather(i)}>×</button></div>{:else}<p class="empty">등록된 날씨가 없습니다.</p>{/each}
	</aside>
	<section class="weather-detail">
		{#if selected}<details class="editor-accordion" open><summary>{selected.name} 설정</summary><div class="editor-accordion-content"><div class="fields"><label>날씨 이름<input bind:value={selected.name} /></label><label>조우율 보정<input type="number" step="0.1" bind:value={selected.encounterRateModifier} /></label><label>시야 보정<input type="number" step="0.1" bind:value={selected.visibilityModifier} /></label></div></div></details>
		<details class="editor-accordion" open><summary>{selected.name}에서의 전환 규칙 ({selectedTransitions.length})</summary><div class="editor-accordion-content transition-content"><button class="add" onclick={addTransition}>+ 전환 규칙 추가</button>
			{#each selectedTransitions as row (`${row.transition.fromWeatherId}-${row.index}`)}{@const transition = row.transition}<div class="transition-card"><span class="from-weather">{selected.name}</span><span class="arrow">→</span><label>전환 날씨<select bind:value={transition.toWeatherId}>{#each weatherTypes as weather (weather.id)}<option value={weather.id}>{weather.name}</option>{/each}</select></label><label>적용 맵<select bind:value={transition.mapId}>{#each maps as map (map.id)}<option value={map.id}>{map.name}</option>{/each}</select></label><label>확률<input type="number" min="0" max="1" step="0.05" bind:value={transition.probability} /></label><button class="remove" aria-label="전환 규칙 삭제" onclick={() => removeTransition(row.index)}>×</button></div>{:else}<p class="empty">이 날씨에서 시작하는 전환 규칙이 없습니다.</p>{/each}
		</div></details>{:else}<div class="empty">날씨를 선택하거나 추가하세요.</div>{/if}
	</section>
</div>

<style>
	.weather-layout{display:grid;grid-template-columns:minmax(230px,290px) minmax(0,1fr);gap:1rem;align-items:start}.weather-list,.weather-detail{display:flex;flex-direction:column;gap:.75rem;min-width:0}.weather-list{padding:1rem;border:1px solid var(--border);border-radius:var(--radius-lg);background:var(--surface)}.weather-list header{display:flex;align-items:center;justify-content:space-between;gap:.5rem}.weather-list h3{margin:0;font-size:.95rem}.weather-item{display:flex;align-items:center;border:1px solid var(--border);border-radius:var(--radius-md);background:var(--surface-2)}.weather-item.active{border-color:var(--accent)}.pick{flex:1;text-align:left;display:flex;flex-direction:column;gap:.2rem;border:0;background:none;color:var(--text);padding:.7rem;cursor:pointer}.pick small,.empty{color:var(--text-muted);font-size:.78rem}.remove{border:0;background:none;color:var(--text-muted);font-size:1.25rem;padding:.45rem .7rem;cursor:pointer}.remove:hover{color:var(--accent)}.add{border:1px solid var(--border);border-radius:var(--radius-md);padding:.5rem .7rem;background:var(--surface-2);color:var(--text);cursor:pointer;font:inherit;font-size:.78rem}.add:hover{background:var(--surface-hover)}.fields{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:.75rem}.fields label,.transition-card label{display:flex;flex-direction:column;gap:.35rem;font-size:.78rem;color:var(--text-muted)}.fields input,.transition-card select,.transition-card input{width:100%;box-sizing:border-box;padding:.55rem;border:1px solid var(--border);border-radius:var(--radius-md);background:var(--bg);color:var(--text);font:inherit}.transition-content{display:flex;flex-direction:column;gap:.65rem}.transition-card{display:grid;grid-template-columns:1fr auto 1fr 1.2fr .8fr auto;gap:.6rem;align-items:end;padding:.7rem;border:1px solid var(--border);border-radius:var(--radius-md);background:var(--surface-2)}.from-weather{align-self:center;color:var(--text);font-weight:650}.arrow{padding-bottom:.5rem;color:var(--text-muted)}@media(max-width:760px){.weather-layout{grid-template-columns:1fr}.fields{grid-template-columns:1fr}.transition-card{grid-template-columns:1fr 1fr}.transition-card .arrow{display:none}}
</style>
