<script lang="ts">
	// 변수 보기·고치기 (5-F-4-1, ADR-029): 이 대화 변수(meta.variables — SillyTavern chat 변수와 같은 범위)와
	// 전역 변수(플러그인 설정 quickReply.globals — /setglobalvar). 프롬프트에서는 {{getvar::이름}}·{{getglobalvar::이름}}
	import { Plus, Trash2 } from "@lucide/svelte";
	import { IconButton, Input } from "$shared/ui";
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
		{ id: "conv", title: "이 대화", macro: "getvar", vars: convVars, save: saveConv },
		{ id: "global", title: "전역", macro: "getglobalvar", vars: globalVars, save: saveGlobal },
	]);

	/** 더하기 칸 (화면 상태) */
	let draft = $state<Record<string, { name: string; value: string }>>({ conv: { name: "", value: "" }, global: { name: "", value: "" } });

	async function setValue(g: (typeof groups)[number], name: string, value: string) {
		if (g.vars[name] === value) return;
		await g.save({ ...g.vars, [name]: value.slice(0, MAX_VALUE) });
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
	{#each groups as g (g.id)}
		<section>
			<h4>{g.title} <span class="faint">{Object.keys(g.vars).length}</span></h4>
			{#if Object.keys(g.vars).length}
				<ul>
					{#each Object.entries(g.vars).sort(([a], [b]) => a.localeCompare(b)) as [name, value] (name)}
						<li>
							<button type="button" class="name" title={`누르면 {{${g.macro}::${name}}} 복사`} onclick={() => copyMacro(g.macro, name)}>{name}</button>
							<Input value={value} aria-label="{g.title} 변수 {name}의 값" maxlength={MAX_VALUE} onchange={(e) => setValue(g, name, e.currentTarget.value)} />
							<IconButton label="{name} 지우기" size="sm" onclick={() => remove(g, name)}><Trash2 /></IconButton>
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
		margin: 0;
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
