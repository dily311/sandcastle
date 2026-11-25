<script lang="ts">
	// 예산 관리 · API별 사용량 · 단가표 (ActivitySection에서 분리)
	import { onDestroy, onMount } from "svelte";
	import { RefreshCw, Trash2 } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import type { ApiProfiles, BudgetOverview_Serialize as BudgetOverview, UsageSummary_Serialize as Usage } from "$ipc/bindings";
	import { Badge, Button, Segmented } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { onEvent } from "$shared/ipc/events";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import PriceTable from "./PriceTable.svelte";
	import BudgetLimits from "./BudgetLimits.svelte";

	const src = { source: "budget" };
	const kindLabel = (k: string) =>
		k === "chat"
			? $_("settings.budgetDetail.kindChat")
			: k === "tts"
				? $_("settings.budgetDetail.kindTts")
				: k === "image"
					? $_("settings.budgetDetail.kindImage")
					: k === "embedding"
						? $_("settings.budgetDetail.kindEmbedding")
						: k;

	let usage = $state<Usage[]>([]);
	let profiles = $state<ApiProfiles[]>([]);
	let budgets = $state<BudgetOverview[]>([]);
	let period = $state<"day" | "week" | "month" | "all">("week");
	let loading = $state(false);
	let showPrices = $state(false);

	const since = (p: typeof period) => {
		const day = 86_400_000;
		return p === "all" ? null : Date.now() - (p === "day" ? day : p === "week" ? 7 * day : 30 * day);
	};
	const costTotals = $derived.by(() => {
		const t = new Map<string, number>();
		for (const u of usage) if (u.estimatedCost != null) t.set(u.currency ?? "$", (t.get(u.currency ?? "$") ?? 0) + u.estimatedCost);
		return [...t.entries()];
	});
	const usedModels = $derived([...new Set(usage.map((u) => u.model).filter((m): m is string => !!m))]);
	const money = (v: number, cur: string) => `${cur}${v < 0.01 && v > 0 ? v.toFixed(4) : v.toFixed(2)}`;
	const n = (v: number) => v.toLocaleString();

	function amount(u: { inputTokens: number; outputTokens: number; characters: number; images: number }) {
		const parts: string[] = [];
		if (u.inputTokens || u.outputTokens) parts.push($_("settings.activityDetail.tokensSummary", { values: { input: n(u.inputTokens), output: n(u.outputTokens) } }));
		if (u.characters) parts.push($_("settings.activityDetail.charsUnit", { values: { count: n(u.characters) } }));
		if (u.images) parts.push($_("settings.activityDetail.imagesUnit", { values: { count: n(u.images) } }));
		return parts.join(" · ");
	}

	async function load() {
		loading = true;
		try {
			[usage, profiles, budgets] = await Promise.all([
				call(commands.usageSummary(since(period)), { ...src, silent: true }),
				call(commands.getAllApiProfile(), { ...src, silent: true }),
				call(commands.listBudgetStatus(), { ...src, silent: true }),
			]);
		} catch {
			toasts.error($_("settings.budgetDetail.loadError"));
		} finally {
			loading = false;
		}
	}

	let firstPeriod = true;
	$effect(() => {
		const p = period;
		if (firstPeriod) {
			firstPeriod = false;
			return;
		}
		void call(commands.usageSummary(since(p)), { ...src, silent: true }).then((u) => (usage = u)).catch(() => {});
	});

	let timer: ReturnType<typeof setTimeout> | undefined;
	let unlisten: (() => void) | undefined;
	onMount(() => {
		void load();
		void onEvent<{ type: string }>("task-event", (e) => {
			if (e.type === "textDelta" || e.type === "progress") return;
			clearTimeout(timer);
			timer = setTimeout(load, 300);
		}).then((u) => (unlisten = u));
	});
	onDestroy(() => {
		clearTimeout(timer);
		unlisten?.();
	});

	async function clearUsage() {
		const ok = await confirmAction({
			title: $_("settings.budgetDetail.clearTitle"),
			message: $_("settings.budgetDetail.clearMsg"),
			danger: true,
			confirmLabel: $_("settings.budgetDetail.clearBtn"),
		});
		if (!ok) return;
		await call(commands.clearUsage(), src);
		toasts.success($_("settings.budgetDetail.clearedToast"));
		void load();
	}
</script>

<Section title={$_("settings.budget.title")} lead={$_("settings.budget.lead")}>
	{#snippet actions()}
		<Button size="sm" disabled={loading} onclick={load}>{#snippet icon()}<RefreshCw />{/snippet}{$_("common.refresh")}</Button>
	{/snippet}

	<Group title={$_("settings.budget.limits")}>
		<BudgetLimits {profiles} items={budgets} onchanged={load} />
	</Group>

	<Group title={$_("settings.budget.usage")}>
		{#snippet aside()}
			<div class="aside">
				<Segmented
					label={$_("settings.budgetDetail.periodAria")}
					bind:value={period}
					options={[
						{ value: "day", label: $_("settings.budget.today") },
						{ value: "week", label: $_("settings.budget.days7") },
						{ value: "month", label: $_("settings.budget.days30") },
						{ value: "all", label: $_("settings.budget.all") },
					]}
				/>
				<Button size="sm" variant="ghost" disabled={!usage.length} onclick={clearUsage}>{#snippet icon()}<Trash2 />{/snippet}{$_("settings.budget.clear")}</Button>
			</div>
		{/snippet}
		{#if costTotals.length}<p class="total">{$_("settings.budgetDetail.estimatedTotal", { values: { total: costTotals.map(([c, v]) => money(v, c)).join(" · ") } })} <span class="muted">{$_("settings.budgetDetail.estimatedTotalHint")}</span></p>{/if}
		{#each usage as u (`${u.apiProfileId}:${u.kind}:${u.model}`)}
			<div class="item">
				<div class="main">
					<span class="label">{u.profileName ?? (u.apiProfileId ? $_("settings.budgetDetail.deletedProfile") : $_("settings.budgetDetail.unknownProfile"))}</span>
					<Badge>{kindLabel(u.kind)}</Badge>
					{#if u.model}<code class="muted">{u.model}</code>{/if}
					<span class="muted">{$_("settings.budgetDetail.callsCount", { values: { count: n(u.calls) } })}{u.failed ? $_("settings.budgetDetail.callsFailed", { values: { count: n(u.failed) } }) : ""}</span>
				</div>
				<div class="amount">
					{amount(u) || "—"}
					{#if u.kind === "chat" && u.estimatedCalls}<span class="muted" title={$_("settings.budgetDetail.estimatedCallsTitle")}>{$_("settings.budgetDetail.estimatedCalls", { values: { count: n(u.estimatedCalls) } })}</span>{/if}
					{#if u.estimatedCost != null}<b class="cost" title={$_("settings.budgetDetail.estimatedCostTitle")}>≈ {money(u.estimatedCost, u.currency ?? "$")}</b>{/if}
				</div>
			</div>
		{:else}
			<p class="empty">{$_("settings.budget.noUsage")}</p>
		{/each}
	</Group>

	<Group title={$_("settings.budget.priceTable")}>
		{#snippet aside()}
			<Button size="sm" variant="ghost" aria-expanded={showPrices} onclick={() => (showPrices = !showPrices)}>{showPrices ? $_("settings.budget.collapse") : $_("settings.budget.expand")}</Button>
		{/snippet}
		{#if showPrices}
			<div class="prices"><PriceTable models={usedModels} onchanged={load} /></div>
		{:else}
			<p class="empty">{$_("settings.budget.priceHint")}</p>
		{/if}
	</Group>
</Section>

<style>
	.item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		padding: var(--space-3) 0;
		flex-wrap: wrap;
	}
	.item + .item {
		border-top: 1px solid var(--border);
	}
	.main {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;
		flex-wrap: wrap;
	}
	.label {
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.muted {
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.amount {
		font-size: var(--text-sm);
		font-variant-numeric: tabular-nums;
	}
	.total {
		padding: var(--space-3) 0 0;
		font-size: var(--text-sm);
	}
	.cost {
		margin-left: var(--space-2);
	}
	.prices {
		padding: var(--space-4) 0;
	}
	.empty {
		padding: var(--space-4) 0;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.aside {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
</style>
