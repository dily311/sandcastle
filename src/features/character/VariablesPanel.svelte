<script lang="ts">
	// 변수 보기·고치기 (5-F-4-1, ADR-029): 이 대화 변수(meta.variables — SillyTavern chat 변수와 같은 범위)와
	// 전역 변수(플러그인 설정 quickReply.globals — /setglobalvar). 프롬프트에서는 {{getvar::이름}}·{{getglobalvar::이름}}
	import UtilMacroNote from "./UtilMacroNote.svelte";
	import { presetOf } from "./presetMacros.svelte";
	import { Plus, Trash2 } from "@lucide/svelte";
	import { Button, Dialog, IconButton, Input, Switch } from "$shared/ui";
	import { ListPlus, SlidersHorizontal } from "@lucide/svelte";
	import VarDefsEditor from "./VarDefsEditor.svelte";
	import type { VarDef } from "./library.svelte";
	import { coerceVar, varRows, varTitle, type VarRow } from "$features/status/vars";
	import type { Conversation } from "./library.svelte";
	import { library } from "./library.svelte";
	import { pluginSetting } from "$shared/stores/aiTools.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { onMount } from "svelte";

	let { conv }: { conv: Conversation } = $props();
	// 전역 변수는 플러그인 설정에 있다 — 탭을 열 때 읽는다 (아직 안 읽었으면)
	onMount(() => {
		if (!pluginSetting.value) void pluginSetting.load();
	});
	// 이 캐릭터의 변수 정의 (5-F-4-3) — 정의된 변수는 값이 없어도 보이고, 종류·범위에 맞춰 고친다
	$effect(() => {
		if (conv.characterId) void library.loadVarDefs(conv.characterId);
	});
	const defs = $derived(library.varDefs[conv.characterId] ?? []);

	// ---- 정의 더하기·지우기 (5-F-6, 사용자 요청): 캐릭터 편집에 가지 않고 여기서. 저장 자리는 캐릭터의 정의와 같다 ----
	let defsOpen = $state(false);
	let draftDefs = $state<VarDef[]>([]);
	let savingDefs = $state(false);
	/** key를 주면 그 이름의 정의를 새로 붙여 연다 (정의 없는 변수 줄의 "정의로") */
	function openDefs(key?: string) {
		draftDefs = defs.map((d) => ({ ...d }));
		if (key && !draftDefs.some((d) => d.key === key)) {
			draftDefs.push({ key, description: "", kind: "string", defaultValue: null, min: null, max: null, maxStep: null, aiRead: true, aiWrite: false });
		}
		defsOpen = true;
	}
	async function saveDefs() {
		savingDefs = true;
		try {
			await library.setVarDefs(conv.characterId, draftDefs);
			defsOpen = false;
		} catch {
			// 이름 겹침 등 이유는 call이 알림으로 — 대화상자는 열어 둔다
		} finally {
			savingDefs = false;
		}
	}

	/** 변수 이름·값 상한 (프롬프트가 부풀지 않게) */
	const MAX_NAME = 64;
	const MAX_VALUE = 4000;

	type Vars = Record<string, string>;
	/** 글자가 아닌 값(스크립트가 넣은 목록·객체)은 JSON 글로 보여 주고, 고치면 글자로 저장 */
	const asText = (v: unknown) => (typeof v === "string" ? v : JSON.stringify(v));
	const toVars = (o: Record<string, unknown> | undefined | null): Vars => Object.fromEntries(Object.entries(o ?? {}).map(([k, v]) => [k, asText(v)]));

	const convVars = $derived(toVars((conv.meta as { variables?: Record<string, unknown> } | null)?.variables));
	const globalVars = $derived(toVars(pluginSetting.value?.quickReply?.globals));

	async function saveConv(next: Vars) {
		await library.patchMeta(conv.id, { variables: next });
	}
	async function saveGlobal(next: Vars) {
		const cur = pluginSetting.value ?? (await pluginSetting.load(), pluginSetting.value);
		if (!cur) throw new Error("설정을 읽지 못해 전역 변수를 저장하지 않음");
		await pluginSetting.save({ ...cur, quickReply: { ...cur.quickReply, globals: next } });
	}
	const groups = $derived([
		{ id: "conv", title: "이 대화", macro: "getvar", vars: convVars, rows: varRows(convVars, defs), save: saveConv },
		{ id: "global", title: "전역", macro: "getglobalvar", vars: globalVars, rows: varRows(globalVars), save: saveGlobal },
	]);

	/** 더하기 칸 (화면 상태) */
	let draft = $state<Record<string, { name: string; value: string }>>({ conv: { name: "", value: "" }, global: { name: "", value: "" } });

	/** 값 고치기. 정의가 있으면 종류·범위에 맞춘다 (맞출 수 없으면 알리고 그대로) */
	async function setValue(g: (typeof groups)[number], row: VarRow, raw: string, input?: HTMLInputElement) {
		const r = coerceVar(row.def, raw.slice(0, MAX_VALUE));
		if ("error" in r) {
			toasts.error(r.error);
			if (input) input.value = row.value ?? "";
			return;
		}
		if (input) input.value = r.value;
		if (g.vars[row.name] === r.value) return;
		await g.save({ ...g.vars, [row.name]: r.value });
	}
	async function add(g: (typeof groups)[number]) {
		const d = draft[g.id];
		const name = d.name.trim().slice(0, MAX_NAME);
		if (!name) return;
		if (name in g.vars && !(await confirmAction({ title: "변수 덮어쓰기", message: `'${name}'이(가) 이미 있음. 값을 바꿈`, confirmLabel: "바꾸기" }))) return;
		try {
			await g.save({ ...g.vars, [name]: d.value.slice(0, MAX_VALUE) });
			draft[g.id] = { name: "", value: "" };
		} catch (e) {
			toasts.error(e instanceof Error ? e.message : String(e));
		}
	}
	async function remove(g: (typeof groups)[number], name: string) {
		if (!(await confirmAction({ title: "변수 지우기", message: `${g.title} 변수 '${name}'을(를) 지움. 되돌릴 수 없음`, confirmLabel: "지우기", danger: true }))) return;
		const { [name]: _gone, ...rest } = g.vars;
		void _gone;
		await g.save(rest);
	}
	async function copyMacro(macro: string, name: string) {
		await navigator.clipboard.writeText(`{{${macro}::${name}}}`).then(
			() => toasts.info(`{{${macro}::${name}}} 복사함`),
			() => {},
		);
	}
</script>

<div class="vars sc-variables">
	<!-- AI가 변수를 읽고 바꾸기 (5-F-4-4): 기기마다, 기본 꺼짐. 요금이 늘어 부연 설명을 둔다 -->
	<Switch
		checked={!!pluginSetting.value?.status?.aiVars}
		label="AI가 변수 읽고 바꾸기"
		description="켜면 응답마다 변수 규칙·지금 값이 프롬프트에 들어가 토큰이 늚. 캐릭터 편집에서 'AI가 읽음·바꿈'을 켠 변수만"
		onchange={(v) => {
			const cur = pluginSetting.value;
			if (cur) void pluginSetting.save({ ...cur, status: { ...cur.status, aiVars: v } });
		}}
	/>
	{#if pluginSetting.value?.status?.aiVars}<UtilMacroNote presetId={presetOf(conv)} macro="vars" />{/if}
	{#each groups as g (g.id)}
		<section>
			<h4>
				{g.title} <span class="faint">{g.rows.length}</span>
				{#if g.id === "conv" && conv.characterId}
					<IconButton label="이 캐릭터의 변수 정의" size="sm" onclick={() => openDefs()}><SlidersHorizontal /></IconButton>
				{/if}
			</h4>
			{#if g.rows.length}
				<ul>
					{#each g.rows as row (row.name)}
						<li class:defined={!!row.def}>
							<button type="button" class="name" title={[varTitle(row.def), `누르면 {{${g.macro}::${row.name}}} 복사`].filter(Boolean).join("\n")} onclick={() => copyMacro(g.macro, row.name)}>{row.name}</button>
							{#if row.def?.kind === "boolean"}
								<Switch checked={(row.value ?? row.def.defaultValue) === "true"} label="{g.title} 변수 {row.name}" hideLabel onchange={(v) => setValue(g, row, String(v))} />
							{:else}
								<Input
									value={row.value ?? ""}
									placeholder={row.def?.defaultValue ?? ""}
									inputmode={row.def?.kind === "number" ? "decimal" : undefined}
									aria-label="{g.title} 변수 {row.name}의 값"
									maxlength={MAX_VALUE}
									onchange={(e) => setValue(g, row, e.currentTarget.value, e.currentTarget)}
								/>
							{/if}
							<span class="row-tools">
								{#if g.id === "conv" && !row.def && conv.characterId}
									<IconButton label="{row.name}을(를) 정의로 (종류·범위·AI 읽기/쓰기)" size="sm" onclick={() => openDefs(row.name)}><ListPlus /></IconButton>
								{/if}
								{#if row.value !== undefined}
									<IconButton label="{row.name} 지우기" size="sm" onclick={() => remove(g, row.name)}><Trash2 /></IconButton>
								{/if}
							</span>
						</li>
					{/each}
				</ul>
			{/if}
			<form
				class="add"
				onsubmit={(e) => {
					e.preventDefault();
					void add(g);
				}}
			>
				<Input bind:value={draft[g.id].name} placeholder="이름" aria-label="{g.title} 새 변수 이름" maxlength={MAX_NAME} />
				<Input bind:value={draft[g.id].value} placeholder="값" aria-label="{g.title} 새 변수 값" maxlength={MAX_VALUE} />
				<IconButton label="{g.title} 변수 더하기" size="sm" type="submit" disabled={!draft[g.id].name.trim()}><Plus /></IconButton>
			</form>
		</section>
	{/each}
</div>

<Dialog bind:open={defsOpen} title="이 캐릭터의 변수 정의" size="lg">
	<VarDefsEditor bind:defs={draftDefs} />
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (defsOpen = false)}>취소</Button>
		<Button variant="primary" loading={savingDefs} onclick={saveDefs}>저장</Button>
	{/snippet}
</Dialog>

<style>
	.vars {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	section {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	h4 {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		margin: 0;
	}
	.row-tools {
		display: flex;
	}
	.faint {
		color: var(--text-faint);
		font-weight: normal;
	}
	ul {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	li,
	.add {
		display: grid;
		grid-template-columns: minmax(0, 2fr) minmax(0, 3fr) auto;
		align-items: center;
		gap: var(--space-1);
	}
	.name {
		overflow: hidden;
		padding: 0;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		text-align: left;
		text-overflow: ellipsis;
		white-space: nowrap;
		cursor: copy;
	}
</style>
