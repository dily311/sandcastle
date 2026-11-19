<script lang="ts">
	// 단가표 (CLAUDE.md 1-A-3, ADR-007): 모델별 단가를 사용자가 적는다. 앱은 가격을 넣어 두지 않는다(가격은 바뀐다).
	// 사용량 × 단가 = "추정" 금액. 실행 기록 화면이 열려 있을 때만 불러온다
	import { onMount } from "svelte";
	import { Plus, Trash2 } from "@lucide/svelte";
	import type { ModelPrice } from "$ipc/bindings";
	import { Button, IconButton, Input } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";

	interface Props {
		/** 사용 기록에 나온 모델 (아직 단가가 없으면 빈 줄로 보여줌) */
		models: string[];
		onchanged?: () => void;
	}
	let { models, onchanged }: Props = $props();
	const src = { source: "pricing" };

	type Row = { model: string; currency: string; input: string; output: string; chars: string; image: string; saved: boolean };
	let saved = $state<ModelPrice[]>([]);
	let drafts = $state<Record<string, Row>>({});
	let newModel = $state("");

	const str = (v: number | null | undefined) => (v == null ? "" : String(v));
	const num = (v: string) => (v.trim() === "" || Number.isNaN(Number(v)) ? null : Number(v));
	function toRow(p: ModelPrice): Row {
		return { model: p.model, currency: p.currency, input: str(p.inputPerMtok), output: str(p.outputPerMtok), chars: str(p.per1kChars), image: str(p.perImage), saved: true };
	}
	const empty = (model: string): Row => ({ model, currency: saved[0]?.currency ?? "$", input: "", output: "", chars: "", image: "", saved: false });

	async function load() {
		saved = await call(commands.listModelPrices(), { ...src, silent: true }).catch(() => []);
		drafts = Object.fromEntries(saved.map((p) => [p.model, toRow(p)]));
	}
	onMount(load);

	// 저장된 단가 + 사용 기록에만 있는 모델
	const rows = $derived.by(() => {
		const out = Object.values(drafts);
		for (const m of models) if (m && !drafts[m]) out.push(empty(m));
		return out.sort((a, b) => Number(b.saved) - Number(a.saved) || a.model.localeCompare(b.model));
	});
	function field(r: Row, key: keyof Omit<Row, "model" | "saved">, v: string) {
		drafts[r.model] = { ...(drafts[r.model] ?? r), [key]: v };
	}
	function dirty(r: Row) {
		const p = saved.find((x) => x.model === r.model);
		if (!p) return [r.input, r.output].some((v) => v.trim() !== "");
		const o = toRow(p);
		return o.currency !== r.currency || o.input !== r.input || o.output !== r.output;
	}
	async function save(r: Row) {
		await call(commands.upsertModelPrice({ model: r.model, currency: r.currency, inputPerMtok: num(r.input), outputPerMtok: num(r.output), per1kChars: num(r.chars), perImage: num(r.image) }), src);
		toasts.success("단가를 저장함");
		await load();
		onchanged?.();
	}
	async function remove(r: Row) {
		if (r.saved) await call(commands.deleteModelPrice(r.model), src);
		await load();
		onchanged?.();
	}
	function add() {
		const m = newModel.trim();
		if (!m) return;
		if (!drafts[m]) drafts[m] = empty(m);
		newModel = "";
	}
</script>

<p class="lead">공급자 요금표를 보고 직접 적는다. 앱은 가격을 넣어 두지 않으며 금액은 모두 <b>추정</b>임. 비운 칸은 0으로 계산.</p>
<div class="table" role="table" aria-label="모델별 단가">
	<div class="head" role="row">
		<span role="columnheader">모델</span>
		<span role="columnheader">통화</span>
		<span role="columnheader">입력 /100만 토큰</span>
		<span role="columnheader">출력 /100만 토큰</span>
		<span></span>
	</div>
	{#each rows as r (r.model)}
		<div class="row" role="row">
			<code class="model" title={r.model}>{r.model}</code>
			<Input class="cur" value={r.currency} maxlength={4} aria-label="통화 기호" oninput={(e) => field(r, "currency", e.currentTarget.value)} />
			<Input type="number" min="0" step="any" value={r.input} aria-label="입력 100만 토큰당" oninput={(e) => field(r, "input", e.currentTarget.value)} />
			<Input type="number" min="0" step="any" value={r.output} aria-label="출력 100만 토큰당" oninput={(e) => field(r, "output", e.currentTarget.value)} />
			<span class="acts">
				<Button size="sm" variant="primary" disabled={!dirty(r)} onclick={() => save(r)}>저장</Button>
				{#if r.saved || drafts[r.model]}<IconButton label="단가 지우기" size="sm" onclick={() => remove(r)}><Trash2 /></IconButton>{/if}
			</span>
		</div>
	{:else}
		<p class="empty">아직 쓴 모델이 없음. 아래에서 모델 이름으로 추가할 수 있음</p>
	{/each}
</div>
<form
	class="add"
	onsubmit={(e) => {
		e.preventDefault();
		add();
	}}
>
	<Input bind:value={newModel} placeholder="모델 이름 (사용 기록의 모델명과 같게)" aria-label="추가할 모델 이름" />
	<Button size="sm" type="submit" disabled={!newModel.trim()}>{#snippet icon()}<Plus />{/snippet}모델 추가</Button>
</form>

<style>
	.lead {
		margin: 0 0 var(--space-3);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.table {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		overflow-x: auto;
	}
	.head,
	.row {
		display: grid;
		grid-template-columns: minmax(120px, 1.4fr) 56px repeat(2, minmax(84px, 1fr)) auto;
		align-items: center;
		gap: var(--space-2);
		min-width: 440px;
	}
	.head {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.model {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--text-sm);
	}
	.acts {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}
	.empty {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.add {
		display: flex;
		gap: var(--space-2);
		margin-top: var(--space-3);
		max-width: var(--page-narrow);
	}
	.add :global(.input) {
		flex: 1;
	}
</style>
