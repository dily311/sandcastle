<script lang="ts">
	// 실행 기록 (CLAUDE.md M2): 지금 실행 중인 작업(멈추기) · 최근 실행 내역(자동 실행 표시)
	// 예산 한도·API별 사용량·단가표는 BudgetSection으로 분리
	import { onDestroy, onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { RefreshCw, Square, Trash2 } from "@lucide/svelte";
	import type { AiCallLog_Serialize as CallLog, TaskHistoryItem_Serialize as HistoryItem, TaskInfo_Serialize as TaskInfo } from "$ipc/bindings";
	import { localizeBackendError, localizeTaskFeature, localizeTaskLabel, localizeTaskStatus } from "$lib/utils/backendI18n";
	import { Badge, Button, EmptyState, Segmented, Select, Switch } from "$shared/ui";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { onEvent } from "$shared/ipc/events";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";

	const src = { source: "activity" };
	const STATUS_TONE: Record<string, "neutral" | "accent" | "success" | "warning" | "danger" | "info"> = {
		pending: "neutral",
		running: "accent",
		completed: "success",
		failed: "danger",
		cancelled: "neutral",
		interrupted: "warning",
	};
	let active = $state<TaskInfo[]>([]);
	let history = $state<HistoryItem[]>([]);
	let who = $state<"all" | "auto">("all");
	let open = $state<string | null>(null);
	// 보낸 글·받은 글 (5-E-20-6-7): 기록이 있는 작업, 펼친 작업의 기록
	let logged = $state<Set<string>>(new Set());
	let calls = $state<Record<string, CallLog[]>>({});
	const logOn = $derived(!!(appSetting.value as { aiCallLog?: boolean } | null)?.aiCallLog);
	async function toggleLog(v: boolean) {
		await appSetting.update({ aiCallLog: v } as never);
		toasts.success(v ? $_("settings.activityDetail.logEnabled") : $_("settings.activityDetail.logDisabled"));
	}
	async function clearLogs() {
		if (!(await confirmAction({ title: $_("settings.activityDetail.clearLogsTitle"), message: $_("settings.activityDetail.clearLogsMsg"), confirmLabel: $_("settings.activityDetail.clearLogsBtn"), danger: true }))) return;
		await call(commands.clearAiCallLogs(), src);
		logged = new Set();
		calls = {};
	}
	async function expand(id: string) {
		open = open === id ? null : id;
		if (open && logged.has(id) && !calls[id]) calls[id] = await call(commands.getAiCallLogs(id), { ...src, silent: true }).catch(() => []);
	}
	/** 보낸 것 (JSON) → 역할·글 목록 */
	function sent(c: CallLog): { role: string; content: string }[] {
		try {
			return (JSON.parse(c.requestJson) as { messages?: { role: string; content: string }[] }).messages ?? [];
		} catch {
			return [];
		}
	}
	const roleLabel = (r: string) => (r === "system" ? $_("settings.activityDetail.roleSystem") : r === "user" ? $_("settings.activityDetail.roleUser") : r === "assistant" ? $_("settings.activityDetail.roleAssistant") : r);
	let loading = $state(false);

	const shown = $derived(who === "auto" ? history.filter((h) => h.runBy === "auto") : history);
	const autoCount = $derived(history.filter((h) => h.runBy === "auto").length);

	async function load() {
		loading = true;
		try {
			[active, history] = await Promise.all([
				call(commands.listActiveTasks(), { ...src, silent: true }),
				call(commands.listTaskHistory(100), { ...src, silent: true }),
			]);
			logged = new Set(await call(commands.listAiCallLogTasks(), { ...src, silent: true }).catch(() => []));
			calls = {};
		} catch {
			toasts.error($_("settings.activityDetail.loadError"));
		} finally {
			loading = false;
		}
	}

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
	const RETENTION = $derived([
		{ value: "", label: $_("settings.activitySec.keepForever") },
		{ value: "30", label: $_("settings.activitySec.days30") },
		{ value: "90", label: $_("settings.activitySec.days90") },
		{ value: "180", label: $_("settings.activitySec.days180") },
		{ value: "365", label: $_("settings.activitySec.year1") },
	]);
	const retention = $derived(appSetting.value?.usageRetentionDays ?? null);
	async function setRetention(v: string) {
		const days = v ? Number(v) : null;
		if (days != null && (retention == null || days < retention)) {
			const ok = await confirmAction({
				title: $_("settings.activityDetail.retentionTitle"),
				message: $_("settings.activityDetail.retentionMsg", { values: { days } }),
				danger: true,
				confirmLabel: $_("settings.activityDetail.retentionConfirm"),
			});
			if (!ok) return;
		}
		await appSetting.update({ usageRetentionDays: days });
		if (days == null) return toasts.success($_("settings.activityDetail.keepForeverToast"));
		const r = await call(commands.compactUsage(), src);
		toasts.success(r.usageRows || r.tasks ? $_("settings.activityDetail.compactedToast", { values: { tasks: n(r.tasks), rows: n(r.usageRows) } }) : $_("settings.activityDetail.nothingToCompact"));
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
		return ms < 1000 ? `${ms}ms` : $_("settings.activityDetail.secUnit", { values: { sec: (ms / 1000).toFixed(1) } });
	}
	/** 작업이 쓴 양 한 줄 */
	function amount(u: { inputTokens: number; outputTokens: number; characters: number; images: number }) {
		const parts: string[] = [];
		if (u.inputTokens || u.outputTokens) parts.push($_("settings.activityDetail.tokensSummary", { values: { input: n(u.inputTokens), output: n(u.outputTokens) } }));
		if (u.characters) parts.push($_("settings.activityDetail.charsUnit", { values: { count: n(u.characters) } }));
		if (u.images) parts.push($_("settings.activityDetail.imagesUnit", { values: { count: n(u.images) } }));
		return parts.join(" · ");
	}
	const featureName = (f: string | null) => localizeTaskFeature(f);
	const status = (h: HistoryItem) => {
		if (h.status === "failed" && h.errorCode === "BUDGET_EXCEEDED") return { label: $_("settings.activitySec.budgetBlocked"), tone: "warning" as const };
		return { label: localizeTaskStatus(h.status), tone: STATUS_TONE[h.status] ?? ("neutral" as const) };
	};
</script>

<Section title={$_("settings.activitySec.title")} lead={$_("settings.activitySec.lead")}>
	{#snippet actions()}
		<Button size="sm" disabled={loading} onclick={load}>{#snippet icon()}<RefreshCw />{/snippet}{$_("common.refresh")}</Button>
	{/snippet}

	<Group title={$_("settings.activitySec.running")}>
		{#each active as t (t.id)}
			<div class="item">
				<div class="main">
					<Badge tone="accent">{featureName(t.feature)}</Badge>
					<span class="label">{localizeTaskLabel(t.label, t.taskType)}</span>
					{#if t.runBy === "auto"}<Badge tone="info">{$_("settings.activitySec.auto")}</Badge>{/if}
					<span class="muted">{when(t.createdAt)}</span>
				</div>
				<Button size="sm" variant="ghost" onclick={() => cancel(t.id)}>{#snippet icon()}<Square />{/snippet}{$_("settings.activitySec.stop")}</Button>
			</div>
		{:else}
			<p class="empty">{$_("settings.activitySec.noRunning")}</p>
		{/each}
	</Group>

	<Group title={$_("settings.activitySec.recent")} flush>
		{#snippet aside()}
			<div class="aside">
			<label class="keep">
				<span class="muted">{$_("settings.activitySec.keepLogs")}</span>
				<Switch checked={logOn} label={$_("settings.activitySec.keepLogs")} hideLabel onchange={toggleLog} />
			</label>
			{#if logged.size}<Button size="sm" variant="ghost" onclick={clearLogs}>{#snippet icon()}<Trash2 />{/snippet}{$_("settings.activitySec.clearLogs")}</Button>{/if}
			<label class="keep"
				><span class="muted">{$_("settings.activitySec.retention")}</span>
				<Select value={retention == null ? "" : String(retention)} options={RETENTION} aria-label={$_("settings.activitySec.retention")} onchange={(e) => setRetention(e.currentTarget.value)} />
			</label>
			<Segmented
				label={$_("settings.activitySec.recent")}
				bind:value={who}
				options={[
					{ value: "all", label: `${$_("common.all")} ${history.length}` },
					{ value: "auto", label: `${$_("settings.activitySec.auto")} ${autoCount}` },
				]}
			/>
			</div>
		{/snippet}
		{#each shown as h (h.id)}
			{@const s = status(h)}
			<div class="row">
				<button class="line" aria-expanded={open === h.id} onclick={() => expand(h.id)}>
					<span class="t">{when(h.createdAt)}</span>
					<Badge tone={s.tone}>{s.label}</Badge>
					<span class="feat">{featureName(h.feature)}</span>
					<span class="label">{localizeTaskLabel(h.label, h.taskType)}</span>
					{#if h.runBy === "auto"}<Badge tone="info">{$_("settings.activitySec.auto")}</Badge>{/if}
					{#if logged.has(h.id)}<Badge>{$_("settings.activitySec.sentTag")}</Badge>{/if}
					<span class="muted end">{amount(h)}</span>
				</button>
				{#if open === h.id}
					<dl class="detail">
						<dt>{$_("settings.activityDetail.taskId")}</dt><dd><code>{h.id}</code></dd>
						<dt>{$_("settings.activityDetail.runBy")}</dt><dd>{h.runBy === "auto" ? $_("settings.activityDetail.runByAuto") : $_("settings.activityDetail.runByUser")}</dd>
						{#if h.profileName || h.model}<dt>{$_("settings.activityDetail.apiModel")}</dt><dd>{h.profileName ?? "—"} · {h.model ?? "—"}</dd>{/if}
						{#if took(h)}<dt>{$_("settings.activityDetail.duration")}</dt><dd>{took(h)}</dd>{/if}
						<dt>{$_("settings.activityDetail.attempts")}</dt><dd>{h.calls > 1 ? $_("settings.activityDetail.attemptsWithCalls", { values: { attempts: h.attemptCount, calls: h.calls } }) : $_("settings.activityDetail.attemptsVal", { values: { attempts: h.attemptCount } })}</dd>
						{#if amount(h)}<dt>{$_("settings.activityDetail.usage")}</dt><dd>{amount(h)}{h.estimated ? $_("settings.activityDetail.estimatedSuffix") : ""}</dd>{/if}
						{#if h.errorMessage}<dt>{$_("settings.activityDetail.error")}</dt><dd class="err">{h.errorCode ? `[${h.errorCode}] ` : ""}{localizeBackendError({ code: h.errorCode ?? "UNKNOWN", message: h.errorMessage })}</dd>{/if}
					</dl>
					{#each calls[h.id] ?? [] as c, i (c.id)}
						<div class="call">
							<div class="call-head">
								<b>{$_("settings.activityDetail.callIndex", { values: { n: i + 1 } })}</b>
								<span class="muted">{c.model}</span>
								{#if c.finishReason}<Badge tone={["length", "max_tokens", "MAX_TOKENS"].includes(c.finishReason) ? "warning" : "neutral"}>{$_("settings.activityDetail.finishReason", { values: { reason: c.finishReason } })}</Badge>{/if}
								{#if c.error}<span class="err">{c.error}</span>{/if}
							</div>
							<details>
								<summary>{$_("settings.activityDetail.sentMessages", { values: { count: sent(c).length } })}</summary>
								{#each sent(c) as m, j (j)}
									<div class="msg"><span class="role">{roleLabel(m.role)}</span><pre>{m.content}</pre></div>
								{/each}
							</details>
							<details open>
								<summary>{$_("settings.activityDetail.receivedText")}</summary>
								<pre>{c.responseText || $_("settings.activityDetail.emptyResponse")}</pre>
							</details>
						</div>
					{/each}
				{/if}
			</div>
		{:else}
			<EmptyState title={$_("settings.activityDetail.emptyTitle")} description={$_("settings.activityDetail.emptyDesc")} />
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
	.keep {
		display: flex;
		align-items: center;
		gap: var(--space-2);
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
	.call {
		margin: 0 var(--space-4) var(--space-3);
		padding: var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		font-size: var(--text-sm);
	}
	.call-head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
		margin-bottom: var(--space-2);
	}
	.call summary {
		cursor: pointer;
		color: var(--text-muted);
		margin: var(--space-1) 0;
	}
	.call pre {
		margin: 0;
		max-height: 320px;
		overflow: auto;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font-family: var(--font-mono, monospace);
		font-size: var(--text-xs);
		background: var(--bg-sunken);
		padding: var(--space-2);
		border-radius: var(--radius-sm);
	}
	.msg {
		margin: var(--space-2) 0;
	}
	.role {
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
</style>
