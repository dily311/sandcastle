<script lang="ts">
	// 캐릭터의 대화 변수 정의 (5-F-4-3, ADR-029): 이름·종류·설명·처음 값·(수) 최소·최대·한 번 폭·AI 읽기/쓰기.
	// 정의는 선택 — 정의 없는 변수도 지금처럼 쓴다. 값은 대화마다(상태 탭). 저장은 캐릭터 편집의 저장 버튼으로
	import { Plus, Trash2 } from "@lucide/svelte";
	import { Button, IconButton, Input, Select, Switch } from "$shared/ui";
	import type { VarDef } from "./library.svelte";

	let { defs = $bindable() }: { defs: VarDef[] } = $props();

	const KINDS = [
		{ value: "number", label: "수" },
		{ value: "string", label: "글자" },
		{ value: "boolean", label: "참거짓" },
	];
	const num = (v: string) => (v.trim() === "" || !Number.isFinite(Number(v)) ? null : Number(v));
	const blank = (): VarDef => ({ key: "", description: "", kind: "number", defaultValue: null, min: null, max: null, maxStep: null, aiRead: true, aiWrite: false });
	function add() {
		defs = [...defs, blank()];
	}
	function remove(i: number) {
		defs = defs.filter((_, j) => j !== i);
	}
	function patch(i: number, p: Partial<VarDef>) {
		defs = defs.map((d, j) => (j === i ? { ...d, ...p, ...(p.aiWrite ? { aiRead: true } : {}) } : d));
	}
</script>

<div class="defs sc-var-defs">
	{#each defs as d, i (i)}
		<div class="def">
			<div class="line">
				<Input value={d.key} placeholder="이름" aria-label="변수 이름" maxlength={64} oninput={(e) => patch(i, { key: e.currentTarget.value })} />
				<Select value={d.kind} options={KINDS} aria-label="{d.key || '변수'} 종류" onchange={(e) => patch(i, { kind: e.currentTarget.value as VarDef["kind"] })} />
				<IconButton label="{d.key || '이 변수'} 정의 지우기" size="sm" onclick={() => remove(i)}><Trash2 /></IconButton>
			</div>
			<Input value={d.description} placeholder="설명 (AI에게도 보임)" aria-label="{d.key || '변수'} 설명" maxlength={500} oninput={(e) => patch(i, { description: e.currentTarget.value })} />
			<div class="line nums">
				{#if d.kind === "boolean"}
					<Select
						value={d.defaultValue ?? ""}
						options={[{ value: "", label: "처음 값 없음" }, { value: "true", label: "처음 값: 참" }, { value: "false", label: "처음 값: 거짓" }]}
						aria-label="{d.key || '변수'} 처음 값"
						onchange={(e) => patch(i, { defaultValue: e.currentTarget.value || null })}
					/>
				{:else}
					<Input value={d.defaultValue ?? ""} placeholder="처음 값" aria-label="{d.key || '변수'} 처음 값" inputmode={d.kind === "number" ? "decimal" : undefined} oninput={(e) => patch(i, { defaultValue: e.currentTarget.value || null })} />
				{/if}
				{#if d.kind === "number"}
					<Input value={d.min ?? ""} placeholder="최소" aria-label="{d.key || '변수'} 최소" inputmode="decimal" oninput={(e) => patch(i, { min: num(e.currentTarget.value) })} />
					<Input value={d.max ?? ""} placeholder="최대" aria-label="{d.key || '변수'} 최대" inputmode="decimal" oninput={(e) => patch(i, { max: num(e.currentTarget.value) })} />
					<Input value={d.maxStep ?? ""} placeholder="한 번 폭" title="AI가 한 응답에서 바꿀 수 있는 크기" aria-label="{d.key || '변수'} 한 번 폭" inputmode="decimal" oninput={(e) => patch(i, { maxStep: num(e.currentTarget.value) })} />
				{/if}
			</div>
			<div class="line ai">
				<Switch checked={d.aiRead} label="AI가 읽음" disabled={d.aiWrite} onchange={(v) => patch(i, { aiRead: v })} />
				<Switch checked={d.aiWrite} label="AI가 바꿈" onchange={(v) => patch(i, { aiWrite: v })} />
			</div>
		</div>
	{/each}
	<Button size="sm" onclick={add}>{#snippet icon()}<Plus />{/snippet}변수 정의 추가</Button>
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
