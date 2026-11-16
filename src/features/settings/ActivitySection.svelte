<script lang="ts">
	// 실행 기록 (CLAUDE.md M2): 지금 실행 중인 작업(멈추기) · 최근 실행 내역(자동 실행 표시) · API별 사용량
	// 화면이 열려 있는 동안만 불러오고 작업 이벤트로 새로 고친다 (Lazy Initialization)
	import { onDestroy, onMount } from "svelte";
	import { RefreshCw, Square, Trash2 } from "@lucide/svelte";
	import type { TaskHistoryItem_Serialize as HistoryItem, TaskInfo_Serialize as TaskInfo, UsageSummary_Serialize as Usage } from "$ipc/bindings";
	import { Badge, Button, EmptyState, Segmented, Select } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { onEvent } from "$shared/ipc/events";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import PriceTable from "./PriceTable.svelte";

	const src = { source: "activity" };
	const FEATURE: Record<string, string> = {
		chat: "대화",
		phone: "폰",
		game: "게임",
		translate: "번역",
		tts: "음성",
		imagegen: "이미지",
		settings: "설정",
	};
	const STATUS: Record<string, { label: string; tone: "neutral" | "accent" | "success" | "warning" | "danger" | "info" }> = {
		pending: { label: "대기", tone: "neutral" },
		running: { label: "실행 중", tone: "accent" },
		completed: { label: "완료", tone: "success" },
		failed: { label: "실패", tone: "danger" },
		cancelled: { label: "멈춤", tone: "neutral" },
		interrupted: { label: "끊김", tone: "warning" },
	};
	const KIND: Record<string, string> = { chat: "텍스트", tts: "음성", image: "이미지" };

	let active = $state<TaskInfo[]>([]);
	let history = $state<HistoryItem[]>([]);
	let usage = $state<Usage[]>([]);
	let period = $state<"day" | "week" | "month" | "all">("week");
	let who = $state<"all" | "auto">("all");
	let open = $state<string | null>(null);
	let loading = $state(false);
	let showPrices = $state(false);

	const since = (p: typeof period) => {
		const day = 86_400_000;
		return p === "all" ? null : Date.now() - (p === "day" ? day : p === "week" ? 7 * day : 30 * day);
	};
	const shown = $derived(who === "auto" ? history.filter((h) => h.runBy === "auto") : history);
	const autoCount = $derived(history.filter((h) => h.runBy === "auto").length);
	// 단가표에 적은 모델의 추정 금액 합계 (통화별)
	const costTotals = $derived.by(() => {
		const t = new Map<string, number>();
		for (const u of usage) if (u.estimatedCost != null) t.set(u.currency ?? "$", (t.get(u.currency ?? "$") ?? 0) + u.estimatedCost);
		return [...t.entries()];
	});
	const usedModels = $derived([...new Set(usage.map((u) => u.model).filter((m): m is string => !!m))]);
	const money = (v: number, cur: string) => `${cur}${v < 0.01 && v > 0 ? v.toFixed(4) : v.toFixed(2)}`;

	async function load() {
		loading = true;
		try {
			[active, history, usage] = await Promise.all([
				call(commands.listActiveTasks(), { ...src, silent: true }),
				call(commands.listTaskHistory(100), { ...src, silent: true }),
				call(commands.usageSummary(since(period)), { ...src, silent: true }),
			]);
		} catch {
			toasts.error("실행 기록을 불러오지 못함");
		} finally {
			loading = false;
		}
	}

	// 기간을 바꾸면 사용량만 다시
	let firstPeriod = true;
	$effect(() => {
		const p = period;
		if (firstPeriod) {
			firstPeriod = false;
			return;
		}
		void call(commands.usageSummary(since(p)), { ...src, silent: true }).then((u) => (usage = u)).catch(() => {});
	});

	// 작업이 생기거나 끝나면 새로 고침 (이벤트가 몰려도 한 번만)
	let timer: ReturnType<typeof setTimeout> | undefined;
	let unlisten: (() => void) | undefined;
	onMount(() => {
		void load();
		void appSetting.load();
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

	async function cancel(id: string) {
		await call(commands.cancelTask(id), src).catch(() => {});
		void load();
	}

	// 기록 보관 (1-A-4): 지난 상세는 일별 합계로 남기고 지운다. 사용량 숫자는 그대로
	const RETENTION = [
		{ value: "", label: "계속 보관" },
		{ value: "30", label: "30일" },
		{ value: "90", label: "90일" },
		{ value: "180", label: "180일" },
		{ value: "365", label: "1년" },
	];
	const retention = $derived(appSetting.value?.usageRetentionDays ?? null);
	async function setRetention(v: string) {
		const days = v ? Number(v) : null;
		if (days != null && (retention == null || days < retention)) {
			const ok = await confirmAction({
				title: "기록 보관 기간",
				message: `${days}일보다 오래된 작업 기록과 사용량 상세를 지움 (앱을 켤 때마다 정리). API별 사용량 합계와 예산 한도 계산은 그대로 남음. 지운 상세는 되돌릴 수 없음`,
				danger: true,
				confirmLabel: "정리하기",
			});
			if (!ok) return;
		}
		await appSetting.update({ usageRetentionDays: days });
		if (days == null) return toasts.success("기록을 계속 보관함");
		const r = await call(commands.compactUsage(), src);
		toasts.success(r.usageRows || r.tasks ? `정리함: 작업 ${n(r.tasks)}개 · 사용량 상세 ${n(r.usageRows)}줄` : "정리할 오래된 기록 없음");
		void load();
	}

	async function clearUsage() {
		const ok = await confirmAction({ title: "사용량 기록 비우기", message: "API별 사용량 합계를 지움. 실행 내역은 남지만 작업별 토큰 수는 사라짐", danger: true, confirmLabel: "비우기" });
		if (!ok) return;
		await call(commands.clearUsage(), src);
		toasts.success("사용량 기록을 비움");
		void load();
	}

	const n = (v: number) => v.toLocaleString();
	const pad = (v: number) => String(v).padStart(2, "0");
	function when(t: number) {
		const d = new Date(t);
		const today = new Date();
		const clock = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
		return d.toDateString() === today.toDateString() ? clock : `${d.getMonth() + 1}/${d.getDate()} ${clock}`;
	}
	function took(h: { startedAt: number | null; finishedAt: number | null }) {
		if (!h.startedAt || !h.finishedAt) return "";
		const ms = h.finishedAt - h.startedAt;
		return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}초`;
	}
	/** 작업이 쓴 양 한 줄 */
	function amount(u: { inputTokens: number; outputTokens: number; characters: number; images: number }) {
		const parts: string[] = [];
		if (u.inputTokens || u.outputTokens) parts.push(`입력 ${n(u.inputTokens)} · 출력 ${n(u.outputTokens)} 토큰`);
		if (u.characters) parts.push(`${n(u.characters)}자`);
		if (u.images) parts.push(`그림 ${n(u.images)}장`);
		return parts.join(" · ");
	}
	const featureName = (f: string | null) => (f ? (FEATURE[f] ?? f) : "기타");
	const status = (h: HistoryItem) => (h.status === "failed" && h.errorCode === "BUDGET_EXCEEDED" ? { label: "한도로 막힘", tone: "warning" as const } : (STATUS[h.status] ?? { label: h.status, tone: "neutral" as const }));
</script>

<Section title="실행 기록" lead="AI·음성·이미지 API를 부른 작업과 사용량. 토큰은 공급자가 알려주면 그 값, 아니면 앱이 추정함">
	{#snippet actions()}
		<Button size="sm" disabled={loading} onclick={load}>{#snippet icon()}<RefreshCw />{/snippet}새로 고침</Button>
	{/snippet}

	<Group title="실행 중">
		{#each active as t (t.id)}
			<div class="item">
				<div class="main">
					<Badge tone="accent">{featureName(t.feature)}</Badge>
					<span class="label">{t.label ?? t.taskType}</span>
					{#if t.runBy === "auto"}<Badge tone="info">자동</Badge>{/if}
					<span class="muted">{when(t.createdAt)}</span>
				</div>
				<Button size="sm" variant="ghost" onclick={() => cancel(t.id)}>{#snippet icon()}<Square />{/snippet}멈추기</Button>
			</div>
		{:else}
			<p class="empty">실행 중인 작업 없음</p>
		{/each}
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

	<Group title="최근 실행" flush>
		{#snippet aside()}
			<div class="aside">
			<label class="keep"
				><span class="muted">보관</span>
				<Select value={retention == null ? "" : String(retention)} options={RETENTION} aria-label="작업·사용량 상세 기록 보관 기간" onchange={(e) => setRetention(e.currentTarget.value)} />
			</label>
			<Segmented
				label="실행 주체"
				bind:value={who}
				options={[
					{ value: "all", label: `전체 ${history.length}` },
					{ value: "auto", label: `자동 ${autoCount}` },
				]}
			/>
			</div>
		{/snippet}
		{#each shown as h (h.id)}
			{@const s = status(h)}
			<div class="row">
				<button class="line" aria-expanded={open === h.id} onclick={() => (open = open === h.id ? null : h.id)}>
					<span class="t">{when(h.createdAt)}</span>
					<Badge tone={s.tone}>{s.label}</Badge>
					<span class="feat">{featureName(h.feature)}</span>
					<span class="label">{h.label ?? h.taskType}</span>
					{#if h.runBy === "auto"}<Badge tone="info">자동</Badge>{/if}
					<span class="muted end">{amount(h)}</span>
				</button>
				{#if open === h.id}
					<dl class="detail">
						<dt>작업 id</dt><dd><code>{h.id}</code></dd>
						<dt>실행 주체</dt><dd>{h.runBy === "auto" ? "자동 설정 (자동 번역·자동 읽기 등)" : "사용자"}</dd>
						{#if h.profileName || h.model}<dt>API · 모델</dt><dd>{h.profileName ?? "—"} · {h.model ?? "—"}</dd>{/if}
						{#if took(h)}<dt>걸린 시간</dt><dd>{took(h)}</dd>{/if}
						<dt>시도</dt><dd>{h.attemptCount}회{h.calls > 1 ? ` · API 호출 ${h.calls}번` : ""}</dd>
						{#if amount(h)}<dt>사용량</dt><dd>{amount(h)}{h.estimated ? " (추정 포함)" : ""}</dd>{/if}
						{#if h.errorMessage}<dt>오류</dt><dd class="err">{h.errorCode ? `[${h.errorCode}] ` : ""}{h.errorMessage}</dd>{/if}
					</dl>
				{/if}
			</div>
		{:else}
			<EmptyState title="실행 기록 없음" description="메시지를 보내거나 번역·음성·이미지를 쓰면 여기 남음" />
		{/each}
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
	.keep {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.total {
		margin: var(--space-3) 0 0;
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
	.row + .row {
		border-top: 1px solid var(--border);
	}
	.line {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		padding: var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		text-align: left;
		cursor: pointer;
		flex-wrap: wrap;
	}
	.line:hover {
		background: var(--bg-hover, var(--bg-elevated));
	}
	.t {
		width: 72px;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}
	.feat {
		color: var(--text-muted);
	}
	.end {
		margin-left: auto;
	}
	.detail {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: var(--space-1) var(--space-3);
		margin: 0 var(--space-2) var(--space-3);
		padding: var(--space-3);
		border-radius: var(--radius-sm);
		background: var(--bg);
		font-size: var(--text-sm);
	}
	.detail dt {
		color: var(--text-muted);
	}
	.detail dd {
		margin: 0;
		word-break: break-all;
	}
	.err {
		color: var(--danger, var(--text));
	}
	@media (max-width: 600px) {
		.end {
			margin-left: 0;
			width: 100%;
		}
	}
</style>
