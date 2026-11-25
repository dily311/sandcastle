<script lang="ts">
	// 예산 관리 · API별 사용량 · 단가표 (ActivitySection에서 분리)
	import { onDestroy, onMount } from "svelte";
	import { RefreshCw, Trash2 } from "@lucide/svelte";
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
	const KIND: Record<string, string> = { chat: "텍스트", tts: "음성", image: "이미지", embedding: "임베딩" };

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
		if (u.inputTokens || u.outputTokens) parts.push(`입력 ${n(u.inputTokens)} · 출력 ${n(u.outputTokens)} 토큰`);
		if (u.characters) parts.push(`${n(u.characters)}자`);
		if (u.images) parts.push(`그림 ${n(u.images)}장`);
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
			toasts.error("예산·사용량을 불러오지 못함");
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
			title: "사용량 기록 비우기",
			message: "API별 사용량 합계를 지움. 실행 내역은 남지만 작업별 토큰 수는 사라짐. 금액 한도를 건 프로필의 이번 달 글 호출 기록은 한도 계산 때문에 남김",
			danger: true,
			confirmLabel: "비우기",
		});
		if (!ok) return;
		await call(commands.clearUsage(), src);
		toasts.success("사용량 기록을 비움");
		void load();
	}
</script>

<Section title="예산 관리" lead="API 프로필별 예산 한도, 사용량 합계, 모델별 단가표. 토큰은 공급자가 알려주면 그 값, 아니면 앱이 추정함">
	{#snippet actions()}
		<Button size="sm" disabled={loading} onclick={load}>{#snippet icon()}<RefreshCw />{/snippet}새로 고침</Button>
	{/snippet}

	<Group title="예산 한도">
		<BudgetLimits {profiles} items={budgets} onchanged={load} />
	</Group>

	<Group title="API별 사용량">
		{#snippet aside()}
			<div class="aside">
				<Segmented
					label="기간"
					bind:value={period}
					options={[
						{ value: "day", label: "오늘" },
						{ value: "week", label: "7일" },
						{ value: "month", label: "30일" },
						{ value: "all", label: "전체" },
					]}
				/>
				<Button size="sm" variant="ghost" disabled={!usage.length} onclick={clearUsage}>{#snippet icon()}<Trash2 />{/snippet}비우기</Button>
			</div>
		{/snippet}
		{#if costTotals.length}<p class="total">추정 금액 {costTotals.map(([c, v]) => money(v, c)).join(" · ")} <span class="muted">(단가표 기준, 실제 청구와 다를 수 있음)</span></p>{/if}
		{#each usage as u (`${u.apiProfileId}:${u.kind}:${u.model}`)}
			<div class="item">
				<div class="main">
					<span class="label">{u.profileName ?? (u.apiProfileId ? "지운 프로필" : "알 수 없음")}</span>
					<Badge>{KIND[u.kind] ?? u.kind}</Badge>
					{#if u.model}<code class="muted">{u.model}</code>{/if}
					<span class="muted">{n(u.calls)}회{u.failed ? ` (실패 ${n(u.failed)})` : ""}</span>
				</div>
				<div class="amount">
					{amount(u) || "—"}
					{#if u.kind === "chat" && u.estimatedCalls}<span class="muted" title="공급자가 사용량을 알려주지 않은 호출은 앱이 글자 수로 추정함">· 추정 {n(u.estimatedCalls)}회</span>{/if}
					{#if u.estimatedCost != null}<b class="cost" title="단가표로 계산한 추정 금액">≈ {money(u.estimatedCost, u.currency ?? "$")}</b>{/if}
				</div>
			</div>
		{:else}
			<p class="empty">이 기간에 쓴 API 없음</p>
		{/each}
	</Group>

	<Group title="단가표">
		{#snippet aside()}
			<Button size="sm" variant="ghost" aria-expanded={showPrices} onclick={() => (showPrices = !showPrices)}>{showPrices ? "접기" : "펼치기"}</Button>
		{/snippet}
		{#if showPrices}
			<div class="prices"><PriceTable models={usedModels} onchanged={load} /></div>
		{:else}
			<p class="empty">모델별 단가를 적으면 사용량 옆에 추정 금액이 나옴</p>
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
