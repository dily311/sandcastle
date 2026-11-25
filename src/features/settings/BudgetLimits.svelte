<script lang="ts">
	// 예산 한도 (ADR-007 추가): API 프로필별 "한 달 금액(달러)" 하나를 슬라이더로 정한다.
	// 글 호출(채팅·임베딩)의 추정 금액을 이번 달 1일부터 더해 넘으면 자동 실행은 막히고, 사용자가 누른 실행은 "이번만 계속"을 물음.
	// 음성·그림은 한도가 없다. NovelAI·ElevenLabs는 구독형이라 한도 대신 계정 잔여만 보여준다.
	import { RefreshCw } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import type { ApiProfiles, BudgetOverview_Serialize as Overview, ModelPriceInfo, QuotaLine } from "$ipc/bindings";
	import { Button, Slider, Spinner } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";

	interface Props {
		profiles: ApiProfiles[];
		items: Overview[];
		onchanged?: () => void;
	}
	let { profiles, items, onchanged }: Props = $props();
	const src = { source: "budget" };

	const MAX_USD = 1000;
	/** 한도를 두지 않고 계정 잔여를 보여주는 공급자 */
	const QUOTA_FORMATS = new Set(["novelai", "elevenlabs"]);

	const byId = $derived(new Map(items.map((o) => [o.limit.apiProfileId, o])));
	const usd = (v: number) => `$${v < 0.01 && v > 0 ? v.toFixed(4) : v.toFixed(2)}`;
	const pct = (used: number, limit: number) => Math.min(100, Math.round((used / limit) * 100));
	const tone = (used: number, limit: number) => (used >= limit ? "over" : used * 100 >= limit * 80 ? "near" : "");

	// 슬라이더 값 (프로필별 문자열: 비어 있으면 한도 없음)
	let drafts = $state<Record<string, string>>({});
	function savedVal(p: ApiProfiles): string {
		const v = byId.get(p.id)?.limit.monthlyUsd;
		return v && v > 0 ? String(v) : "";
	}
	function valOf(p: ApiProfiles): string {
		return p.id in drafts ? drafts[p.id] : savedVal(p);
	}

	async function commitValue(p: ApiProfiles, raw: string) {
		const trimmed = raw.trim();
		const parsed = trimmed === "" ? null : Math.min(MAX_USD, Math.max(1, Number(trimmed)));
		const monthlyUsd = parsed != null && Number.isFinite(parsed) ? parsed : null;
		drafts[p.id] = monthlyUsd != null ? String(monthlyUsd) : "";
		const limit = { apiProfileId: p.id, monthlyUsd, legacyMonthlyTokens: null };
		await call(commands.upsertBudgetLimit(limit), src);
		delete drafts[p.id];
		drafts = { ...drafts };
		onchanged?.();
	}

	// ---- 모델 단가 (슬라이더 아래 환산과 "단가 없음" 안내) ----
	let prices = $state<Record<string, ModelPriceInfo | null>>({});
	$effect(() => {
		for (const p of profiles) {
			if (QUOTA_FORMATS.has(p.formatType) || p.model in prices) continue;
			prices[p.model] = null;
			void call(commands.getModelPrice(p.model), { ...src, silent: true })
				.then((r) => (prices[p.model] = r))
				.catch(() => {});
		}
	});
	/** 이 한도면 출력만 썼을 때 대략 몇 토큰인지 */
	function tokensHint(price: ModelPriceInfo | null | undefined, limit: number): string {
		const out = price?.output ?? 0;
		if (!price || out <= 0) return "";
		const n = (limit / out) * 1_000_000;
		return `출력만 쓰면 약 ${n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : `${Math.round(n / 10_000) / 100}M`} 토큰`;
	}

	// ---- 구독형 공급자의 잔여 (누를 때만 읽음) ----
	let quotas = $state<Record<string, QuotaLine[]>>({});
	let loadingQuota = $state<string | null>(null);
	async function loadQuota(p: ApiProfiles) {
		loadingQuota = p.id;
		try {
			quotas[p.id] = await call(commands.getProviderQuota(p.id), src);
		} catch {
			toasts.error("잔여를 읽지 못함");
		} finally {
			loadingQuota = null;
		}
	}
</script>

{#each profiles as p (p.id)}
	{@const o = byId.get(p.id)}
	{@const curStr = valOf(p)}
	{@const curVal = Number(curStr)}
	{@const hasLimit = curStr.trim() !== "" && Number.isFinite(curVal) && curVal > 0}
	<div class="prof">
		<div class="head">
			<span class="name">{p.name}</span>
			<span class="model">{p.model}</span>
		</div>
		{#if QUOTA_FORMATS.has(p.formatType)}
			<div class="quota">
				{#if quotas[p.id]?.length}
					<dl>
						{#each quotas[p.id] as l (l.label)}<dt>{l.label}</dt><dd>{l.value}</dd>{/each}
					</dl>
				{:else}
					<p class="none">구독형이라 한도 대신 계정에 남은 양을 보여줌</p>
				{/if}
				<Button size="sm" variant="ghost" disabled={loadingQuota === p.id} onclick={() => loadQuota(p)}>
					{#snippet icon()}{#if loadingQuota === p.id}<Spinner />{:else}<RefreshCw />{/if}{/snippet}잔여 확인
				</Button>
			</div>
		{:else}
			<div class="slider-wrap">
				<Slider
					label={$_("settings.budget.monthlyBudget")}
					bind:value={() => valOf(p), (v) => (drafts[p.id] = v)}
					min={1}
					max={MAX_USD}
					step={1}
					fallback={100}
					unsetLabel={$_("settings.budget.noLimit")}
					format={(n) => `$${n} / mo`}
					oncommit={(v) => void commitValue(p, v)}
				/>
			</div>
			{#if hasLimit}
				{#if prices[p.model]}
					<p class="hint">{tokensHint(prices[p.model], curVal)}{prices[p.model]?.builtin ? " · 앱의 기본 단가 기준(추정)" : " · 내가 적은 단가 기준"}</p>
				{:else if p.model in prices}
					<p class="warn">이 모델은 단가를 몰라 금액을 셀 수 없음. 아래 "단가표"에 적어야 한도가 걸림</p>
				{/if}
			{/if}
			{#if o?.limit.legacyMonthlyTokens}
				<p class="warn">예전 한 달 토큰 한도({o.limit.legacyMonthlyTokens.toLocaleString()})는 더 이상 쓰지 않음. 위에서 금액 한도를 정하면 지워짐</p>
			{/if}
			{#if o?.meter}
				{@const m = { usedUsd: o.meter.usedUsd ?? 0, limitUsd: o.meter.limitUsd ?? 0, remainingUsd: o.meter.remainingUsd ?? 0, unpricedCalls: o.meter.unpricedCalls }}
				<div class="meter">
					<span class="bar {tone(m.usedUsd, m.limitUsd)}" role="meter" aria-label="이번 달 사용 금액" aria-valuemin={0} aria-valuemax={m.limitUsd} aria-valuenow={Math.min(m.usedUsd, m.limitUsd)}><i style:width="{pct(m.usedUsd, m.limitUsd)}%"></i></span>
					<span class="num">{usd(m.usedUsd)} / {usd(m.limitUsd)}{m.usedUsd >= m.limitUsd ? " · 넘음" : ` · 남은 ${usd(m.remainingUsd)}`}</span>
				</div>
				{#if m.unpricedCalls}<p class="warn">단가를 몰라 금액에 못 넣은 호출 {m.unpricedCalls}건이 있음</p>{/if}
			{/if}
		{/if}
	</div>
{:else}
	<p class="none">API 프로필이 없음</p>
{/each}

<style>
	.prof {
		padding: var(--space-3) 0;
	}
	.prof + .prof {
		border-top: 1px solid var(--border);
	}
	.head {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
		min-width: 0;
	}
	.name {
		font-weight: 500;
	}
	.model {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.slider-wrap {
		margin-top: var(--space-2);
	}
	.hint,
	.warn {
		margin: var(--space-1) 0 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.warn {
		color: var(--warning);
	}
	.meter {
		display: grid;
		grid-template-columns: 1fr auto;
		align-items: center;
		gap: var(--space-3);
		margin-top: var(--space-2);
		font-size: var(--text-sm);
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
		padding: var(--space-2) 0;
		color: var(--text-faint);
		font-size: var(--text-sm);
	}
	.quota {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-1);
		margin-top: var(--space-1);
	}
	.quota dl {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: 2px var(--space-3);
		margin: var(--space-1) 0;
		font-size: var(--text-sm);
	}
	.quota dt {
		color: var(--text-faint);
	}
	.quota dd {
		margin: 0;
		overflow-wrap: anywhere;
	}
	@media (max-width: 600px) {
		.meter {
			grid-template-columns: 1fr;
		}
	}
</style>
