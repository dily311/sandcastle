<script lang="ts">
	// 예산 한도 (CLAUDE.md 1-A-5, ADR-007): API 프로필별 하루/한 달 상한과 지금 쓴 양(막대).
	// 넘으면 자동 실행은 막히고, 사용자가 누른 실행은 "이번만 계속"을 물음. 비우면 한도 없음(기본)
	import type { ApiProfiles, BudgetLimit_Deserialize as BudgetLimit, BudgetMeter, BudgetOverview_Serialize as Overview } from "$ipc/bindings";
	import { Button, Dialog, Input } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";

	interface Props {
		profiles: ApiProfiles[];
		items: Overview[];
		onchanged?: () => void;
	}
	let { profiles, items, onchanged }: Props = $props();
	const src = { source: "budget" };

	const METRIC: Record<BudgetMeter["metric"], string> = { tokens: "토큰", characters: "음성 글자", images: "그림" };
	const PERIOD: Record<BudgetMeter["period"], string> = { day: "오늘", month: "이번 달" };
	const FIELDS = [
		{ key: "dailyTokens", label: "하루 토큰" },
		{ key: "monthlyTokens", label: "한 달 토큰" },
		{ key: "dailyCharacters", label: "하루 음성 글자" },
		{ key: "monthlyCharacters", label: "한 달 음성 글자" },
		{ key: "dailyImages", label: "하루 그림 장 수" },
		{ key: "monthlyImages", label: "한 달 그림 장 수" },
	] as const;
	type Key = (typeof FIELDS)[number]["key"];

	const byId = $derived(new Map(items.map((o) => [o.limit.apiProfileId, o])));
	const n = (v: number) => v.toLocaleString();
	const pct = (m: BudgetMeter) => Math.min(100, Math.round((m.used / m.limit) * 100));
	const tone = (m: BudgetMeter) => (m.used >= m.limit ? "over" : m.used * 100 >= m.limit * 80 ? "near" : "");

	// ---- 편집 ----
	let open = $state(false);
	let editing = $state<ApiProfiles | null>(null);
	let draft = $state<Record<Key, string>>({ dailyTokens: "", monthlyTokens: "", dailyCharacters: "", monthlyCharacters: "", dailyImages: "", monthlyImages: "" });
	function edit(p: ApiProfiles) {
		const l = byId.get(p.id)?.limit;
		editing = p;
		for (const f of FIELDS) draft[f.key] = l?.[f.key] != null ? String(l[f.key]) : "";
		open = true;
	}
	const num = (v: string) => {
		const x = Math.floor(Number(v));
		return v.trim() === "" || !Number.isFinite(x) || x <= 0 ? null : x;
	};
	async function save() {
		if (!editing) return;
		const limit = { apiProfileId: editing.id } as BudgetLimit;
		for (const f of FIELDS) limit[f.key] = num(draft[f.key]);
		await call(commands.upsertBudgetLimit(limit), src);
		toasts.success("한도를 저장함");
		open = false;
		onchanged?.();
	}
	function clearAll() {
		for (const f of FIELDS) draft[f.key] = "";
	}
</script>

{#each profiles as p (p.id)}
	{@const o = byId.get(p.id)}
	<div class="prof">
		<div class="head">
			<span class="name">{p.name}</span>
			<Button size="sm" variant="ghost" onclick={() => edit(p)}>{o ? "한도 바꾸기" : "한도 정하기"}</Button>
		</div>
		{#if o?.meters.length}
			<ul class="meters">
				{#each o.meters as m (`${m.metric}:${m.period}`)}
					<li>
						<span class="what">{PERIOD[m.period]} {METRIC[m.metric]}</span>
						<span class="bar {tone(m)}" role="meter" aria-label="{PERIOD[m.period]} {METRIC[m.metric]} 사용" aria-valuemin={0} aria-valuemax={m.limit} aria-valuenow={Math.min(m.used, m.limit)}><i style:width="{pct(m)}%"></i></span>
						<span class="num">{n(m.used)} / {n(m.limit)}{m.used >= m.limit ? " · 넘음" : ""}</span>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="none">한도 없음</p>
		{/if}
	</div>
{:else}
	<p class="none">API 프로필이 없음</p>
{/each}

<Dialog bind:open title={editing ? `한도 · ${editing.name}` : "한도"} description="비운 칸은 한도 없음. 하루는 오늘 0시, 한 달은 1일 0시부터. 넘으면 자동 실행은 막히고, 직접 누른 실행은 '이번만 계속'을 물음" size="sm">
	<div class="grid">
		{#each FIELDS as f (f.key)}
			<label class="field"><span>{f.label}</span><Input type="number" min="1" step="1" bind:value={draft[f.key]} placeholder="없음" /></label>
		{/each}
	</div>
	{#snippet footer()}
		<Button variant="ghost" onclick={clearAll}>모두 비우기</Button>
		<Button onclick={() => (open = false)}>취소</Button>
		<Button variant="primary" onclick={save}>저장</Button>
	{/snippet}
</Dialog>

<style>
	.prof {
		padding: var(--space-3) 0;
	}
	.prof + .prof {
		border-top: 1px solid var(--border);
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
	}
	.name {
		font-weight: 500;
	}
	.meters {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: var(--space-2) 0 0;
		padding: 0;
		list-style: none;
	}
	.meters li {
		display: grid;
		grid-template-columns: 110px 1fr auto;
		align-items: center;
		gap: var(--space-3);
		font-size: var(--text-sm);
	}
	.what {
		color: var(--text-muted);
	}
	.bar {
		height: 8px;
		border-radius: var(--radius-full, 999px);
		background: var(--bg-sunken);
		overflow: hidden;
	}
	.bar i {
		display: block;
		height: 100%;
		background: var(--accent);
	}
	.bar.near i {
		background: var(--warning);
	}
	.bar.over i {
		background: var(--danger);
	}
	.num {
		font-variant-numeric: tabular-nums;
	}
	.none {
		margin: var(--space-1) 0 0;
		padding: var(--space-4) 0;
		color: var(--text-faint);
		font-size: var(--text-sm);
	}
	.grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	@media (max-width: 600px) {
		.meters li {
			grid-template-columns: 1fr auto;
		}
		.bar {
			grid-column: 1 / -1;
			grid-row: 2;
		}
	}
</style>
