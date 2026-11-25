<script lang="ts">
	// 캐릭터의 대화 변수 정의 (5-F-4-3, ADR-029): 이름·종류·설명·처음 값·(수) 최소·최대·한 번 폭·AI 읽기/쓰기.
	// 정의는 선택 — 정의 없는 변수도 지금처럼 쓴다. 값은 대화마다(상태 탭). 저장은 캐릭터 편집의 저장 버튼으로
	import { Plus, Trash2 } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import { Button, IconButton, Input, Select, Switch } from "$shared/ui";
	import type { VarDef } from "./library.svelte";

	let { defs = $bindable() }: { defs: VarDef[] } = $props();

	const KINDS = $derived([
		{ value: "number", label: $_("finalPolish.varKindNumber") },
		{ value: "string", label: $_("finalPolish.varKindString") },
		{ value: "boolean", label: $_("finalPolish.varKindBoolean") },
	]);
	const num = (v: string) => (v.trim() === "" || !Number.isFinite(Number(v)) ? null : Number(v));
	const blank = (): VarDef => ({ key: "", description: "", kind: "number", defaultValue: null, min: null, max: null, maxStep: null, aiRead: true, aiWrite: false });
	function add() {
		defs = [...defs, blank()];
	}
	function remove(i: number) {
		defs = defs.filter((_item, j) => j !== i);
	}
	function patch(i: number, p: Partial<VarDef>) {
		defs = defs.map((d, j) => (j === i ? { ...d, ...p, ...(p.aiWrite ? { aiRead: true } : {}) } : d));
	}
</script>

<div class="defs sc-var-defs">
	{#each defs as d, i (i)}
		{@const vname = d.key || $_("finalPolish.varFallback")}
		<div class="def">
			<div class="line">
				<Input value={d.key} placeholder={$_("finalPolish.varNamePlaceholder")} aria-label={$_("finalPolish.varNameAria")} maxlength={64} oninput={(e) => patch(i, { key: e.currentTarget.value })} />
				<Select value={d.kind} options={KINDS} aria-label={$_("finalPolish.varKindAria", { values: { name: vname } })} onchange={(e) => patch(i, { kind: e.currentTarget.value as VarDef["kind"] })} />
				<IconButton label={$_("finalPolish.varDeleteAria", { values: { name: d.key || $_("finalPolish.varThisFallback") } })} size="sm" onclick={() => remove(i)}><Trash2 /></IconButton>
			</div>
			<Input value={d.description} placeholder={$_("finalPolish.varDescPlaceholder")} aria-label={$_("finalPolish.varDescAria", { values: { name: vname } })} maxlength={500} oninput={(e) => patch(i, { description: e.currentTarget.value })} />
			<div class="line nums">
				{#if d.kind === "boolean"}
					<Select
						value={d.defaultValue ?? ""}
						options={[{ value: "", label: $_("finalPolish.varDefaultNone") }, { value: "true", label: $_("finalPolish.varDefaultTrue") }, { value: "false", label: $_("finalPolish.varDefaultFalse") }]}
						aria-label={$_("finalPolish.varDefaultAria", { values: { name: vname } })}
						onchange={(e) => patch(i, { defaultValue: e.currentTarget.value || null })}
					/>
				{:else}
					<Input value={d.defaultValue ?? ""} placeholder={$_("finalPolish.varDefaultPlaceholder")} aria-label={$_("finalPolish.varDefaultAria", { values: { name: vname } })} inputmode={d.kind === "number" ? "decimal" : undefined} oninput={(e) => patch(i, { defaultValue: e.currentTarget.value || null })} />
				{/if}
				{#if d.kind === "number"}
					<Input value={d.min ?? ""} placeholder={$_("finalPolish.varMinPlaceholder")} aria-label={$_("finalPolish.varMinAria", { values: { name: vname } })} inputmode="decimal" oninput={(e) => patch(i, { min: num(e.currentTarget.value) })} />
					<Input value={d.max ?? ""} placeholder={$_("finalPolish.varMaxPlaceholder")} aria-label={$_("finalPolish.varMaxAria", { values: { name: vname } })} inputmode="decimal" oninput={(e) => patch(i, { max: num(e.currentTarget.value) })} />
					<Input value={d.maxStep ?? ""} placeholder={$_("finalPolish.varMaxStepPlaceholder")} title={$_("finalPolish.varMaxStepTitle")} aria-label={$_("finalPolish.varMaxStepAria", { values: { name: vname } })} inputmode="decimal" oninput={(e) => patch(i, { maxStep: num(e.currentTarget.value) })} />
				{/if}
			</div>
			<div class="line ai">
				<Switch checked={d.aiRead} label={$_("finalPolish.varAiRead")} disabled={d.aiWrite} onchange={(v) => patch(i, { aiRead: v })} />
				<Switch checked={d.aiWrite} label={$_("finalPolish.varAiWrite")} onchange={(v) => patch(i, { aiWrite: v })} />
			</div>
		</div>
	{/each}
	<Button size="sm" onclick={add}>{#snippet icon()}<Plus />{/snippet}{$_("finalPolish.varAddBtn")}</Button>
</div>

<style>
	.defs {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		align-items: flex-start;
	}
	.def {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		width: 100%;
		padding: var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
	}
	.line {
		display: grid;
		grid-template-columns: minmax(0, 2fr) minmax(0, 1fr) auto;
		align-items: center;
		gap: var(--space-2);
	}
	.line.nums {
		grid-template-columns: repeat(auto-fit, minmax(80px, 1fr));
	}
	.line.ai {
		display: flex;
		gap: var(--space-4);
	}
</style>
