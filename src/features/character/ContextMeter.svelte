<script lang="ts">
	import { _ } from "svelte-i18n";
	// 컨텍스트 사용량 막대: 시스템(규칙·캐릭터·월드인포·노트) / 대화 기록 / 응답 몫 / 남은 칸 + 예상 비용
	import type { ContextUsage, ModelPriceInfo } from "$ipc/bindings";

	let {
		usage,
		basis = "current",
		model = null,
		price = undefined,
		draftTokens = 0,
	}: {
		usage: ContextUsage | null;
		basis?: "current" | "last";
		model?: string | null;
		/** undefined = 조회 전/생략, null = 단가 없음 */
		price?: ModelPriceInfo | null;
		/** 입력칸에 작성 중인 글의 추정 토큰 */
		draftTokens?: number;
	} = $props();

	const n = (v: number) => v.toLocaleString();
	function fmtUsd(v: number): string {
		if (v <= 0) return "$0";
		if (v < 0.0001) return "< $0.0001";
		if (v < 0.01) return `$${v.toFixed(4)}`;
		if (v < 1) return `$${v.toFixed(3)}`;
		return `$${v.toFixed(2)}`;
	}
	/** 메모리 회상을 건너뛴 이유 (백엔드 service/memory/recall.rs의 코드) */
	const RECALL_REASON = $derived<Record<string, string>>({
		short: $_("contextMeter.recall.short"),
		no_index: $_("contextMeter.recall.no_index"),
		other_model: $_("contextMeter.recall.other_model"),
		no_candidates: $_("contextMeter.recall.no_candidates"),
		no_relevant: $_("contextMeter.recall.no_relevant"),
		timeout: $_("contextMeter.recall.timeout"),
		budget: $_("contextMeter.recall.budget"),
		cancelled: $_("contextMeter.recall.cancelled"),
		error: $_("contextMeter.recall.error"),
		regenerate: $_("contextMeter.recall.regenerate"),
	});
	const pct = (v: number) => (usage && usage.limit ? `${Math.min(100, (v / usage.limit) * 100)}%` : "0%");
	const total = $derived(usage?.used != null ? usage.used + usage.reserved : 0);
	const ratio = $derived(usage && usage.limit ? total / usage.limit : 0);
	const tone = $derived(ratio >= 0.95 ? "danger" : ratio >= 0.8 ? "warn" : "ok");

	const inputTokens = $derived(usage?.used != null ? usage.used + Math.max(0, draftTokens) : 0);
	const outputTokens = $derived(usage?.reserved ?? 0);
	const inputRate = $derived(price?.input ?? 0);
	const outputRate = $derived(price?.output ?? 0);
	const inputCost = $derived(price ? (inputTokens * inputRate) / 1_000_000 : 0);
	const maxOutputCost = $derived(price ? (outputTokens * outputRate) / 1_000_000 : 0);
	const maxTotalCost = $derived(inputCost + maxOutputCost);
</script>

<div class="meter" aria-label={$_("contextMeter.aria")}>
	{#if !usage}
		<p class="faint">{$_("contextMeter.loading")}</p>
	{:else if usage.used == null}
		<div class="bar"><i class="reserved" style:width={pct(usage.reserved)}></i></div>
		<p class="faint">{basis === "last" ? $_("contextMeter.notSentYet") : ""}{$_("contextMeter.limitLabel", { values: { val: usage.limit ? $_("contextMeter.tokensUnit", { values: { count: n(usage.limit) } }) : $_("contextMeter.unlimited") } })}</p>
	{:else}
		<div class="nums">
			<b class={tone}>{n(total)}</b><span>/ {usage.limit ? $_("contextMeter.tokensUnit", { values: { count: n(usage.limit) } }) : $_("contextMeter.unlimited")}</span><em>{Math.round(ratio * 100)}%</em>
		</div>
		<div class="bar" role="img" aria-label={$_("contextMeter.barAria", { values: { system: usage.system, history: usage.history, reserved: usage.reserved, limit: usage.limit } })}>
			<i class="system" style:width={pct(usage.system)}></i>
			<i class="history" style:width={pct(usage.history)}></i>
			<i class="reserved" style:width={pct(usage.reserved)}></i>
		</div>
		<ul class="legend">
			<li><span class="dot system"></span>{$_("contextMeter.legendSystem", { values: { count: n(usage.system) } })}</li>
			<li><span class="dot history"></span>{$_("contextMeter.legendHistory", { values: { count: n(usage.history) } })} <small>{$_("contextMeter.legendMsgs", { values: { count: usage.historyCount } })}</small></li>
			{#if usage.reserved}<li><span class="dot reserved"></span>{$_("contextMeter.legendReserved", { values: { count: n(usage.reserved) } })}</li>{/if}
		</ul>
		{#if usage.plugins?.length}
			<!-- 커뮤니티 플러그인이 더한 블록 (6-D-5) -->
			<p class="faint small plugins">{$_("contextMeter.addedByPlugins", { values: { items: usage.plugins.map((p) => `${p.name} ${n(p.tokens)}`).join(" · ") } })} <small>({basis === "last" ? $_("contextMeter.inSystem") : $_("contextMeter.lastReply")})</small></p>
		{/if}
		{#if usage.summary}
			<p class="faint small plugins">{$_("contextMeter.summaryLine", { values: { tokens: n(usage.summary.tokens) } })} <small>{$_("contextMeter.summaryCovered", { values: { count: usage.summary.coveredCount } })}</small></p>
		{/if}
		{#if usage.memoryRecall}
			<!-- 자동 회상 (3-C-5): 지난 대화에서 가져온 글 -->
			{#if usage.memoryRecall.status === "used"}
				<p class="faint small plugins">{$_("contextMeter.memoryRecalled", { values: { items: usage.memoryRecall.hits.map((h) => `${h.conversationTitle || $_("contextMeter.untitled")} ${n(h.tokens)}`).join(" · ") } })} <small>({basis === "last" ? $_("contextMeter.inChat") : $_("contextMeter.lastReply")} · {$_("contextMeter.totalTokens", { values: { count: n(usage.memoryRecall.tokens) } })})</small></p>
			{:else}
				<p class="faint small plugins">{$_("contextMeter.memorySkipped", { values: { reason: RECALL_REASON[usage.memoryRecall.reason ?? ""] ?? usage.memoryRecall.reason ?? "" } })}{#if basis === "current"} <small>({$_("contextMeter.lastReply")})</small>{/if}</p>
			{/if}
		{/if}
		{#if usage.limit && ratio > 1}
			<p class="note">{$_("contextMeter.overLimitNote")}</p>
		{:else if ratio >= 0.8}
			<p class="note">{$_("contextMeter.nearLimitNote")}</p>
		{/if}
	{/if}
	<p class="faint small">{basis === "last" ? $_("contextMeter.basisLast") : $_("contextMeter.basisCurrent")}{usage?.presetName ? $_("contextMeter.presetSuffix", { values: { name: usage.presetName } }) : ""}</p>
	{#if basis === "current" && usage?.used != null && model}
		<div class="cost" aria-label={$_("contextMeter.costAria")}>
			{#if price}
				<div class="cost-head">
					<span>{$_("contextMeter.costAria")} <small>{$_("contextMeter.costPerRun")}</small></span>
					<b>
						{fmtUsd(inputCost)}{#if outputTokens > 0 && outputRate > 0} ~ {fmtUsd(maxTotalCost)}{/if}
					</b>
				</div>
				<p class="faint small">
					{$_("contextMeter.costInput", { values: { tokens: n(inputTokens), cost: fmtUsd(inputCost) } })}{#if outputTokens > 0 && outputRate > 0}{$_("contextMeter.costOutput", { values: { tokens: n(outputTokens), cost: fmtUsd(maxOutputCost) } })}{/if}
				</p>
				<p class="faint small">
					{model} · ${inputRate}/${outputRate} / 1M{price.builtin ? $_("contextMeter.priceBuiltin") : $_("contextMeter.priceCustom")}
				</p>
			{:else if price === null}
				<div class="cost-head">
					<span>{$_("contextMeter.costAria")}</span>
					<span class="faint small">{$_("contextMeter.noPrice")}</span>
				</div>
				<p class="faint small">{$_("contextMeter.noPriceDesc", { values: { model } })}</p>
			{/if}
		</div>
	{/if}
</div>

<style>
	.meter {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.nums {
		display: flex;
		align-items: baseline;
		gap: var(--space-1);
	}
	.nums b {
		font-size: var(--text-lg);
		font-variant-numeric: tabular-nums;
	}
	.nums span {
		color: var(--text-muted);
	}
	.nums em {
		margin-left: auto;
		font-style: normal;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}
	.warn {
		color: var(--warning, #c98a1b);
	}
	.danger {
		color: var(--danger, #d9534f);
	}
	.bar {
		display: flex;
		height: 8px;
		border-radius: 4px;
		background: var(--bg-sunken);
		overflow: hidden;
	}
	.bar i {
		display: block;
		height: 100%;
	}
	.system {
		background: color-mix(in oklab, var(--accent) 55%, var(--text-faint));
	}
	.history {
		background: var(--accent);
	}
	.reserved {
		background: repeating-linear-gradient(135deg, var(--border-strong) 0 3px, transparent 3px 6px);
	}
	.legend {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-wrap: wrap;
		gap: 2px var(--space-3);
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.legend li {
		display: flex;
		align-items: center;
		gap: 5px;
	}
	.dot {
		width: 8px;
		height: 8px;
		border-radius: 2px;
	}
	.note {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	p {
		margin: 0;
	}
	.small {
		font-size: var(--text-xs);
	}
	.cost {
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin-top: var(--space-1);
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--bg-sunken);
	}
	.cost-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-2);
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.cost-head b {
		font-size: var(--text-sm);
		color: var(--text);
		font-variant-numeric: tabular-nums;
	}
</style>
