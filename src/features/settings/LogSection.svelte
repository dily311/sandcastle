<script lang="ts">
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { Copy, Trash2 } from "@lucide/svelte";
	import type { PanicEntry } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { Badge, Button, EmptyState, Segmented } from "$shared/ui";
	import { logs, type LogEntry, type LogLevel } from "$shared/stores/log.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import SyncRunsGroup from "./SyncRunsGroup.svelte";

	let level = $state<"all" | LogLevel>("all");
	let open = $state<number | null>(null);
	const shown = $derived(level === "all" ? logs.entries : logs.entries.filter((e) => e.level === level));

	const pad = (n: number) => String(n).padStart(2, "0");
	const clock = (t: number) => {
		const d = new Date(t);
		return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
	};
	const tone = (l: LogLevel) => (l === "error" ? "danger" : l === "warn" ? "warning" : "neutral");
	function detail(e: LogEntry) {
		if (e.details === undefined || e.details === null) return null;
		try {
			return typeof e.details === "string" ? e.details : JSON.stringify(e.details, null, 2);
		} catch {
			return String(e.details);
		}
	}
	/** 한 줄 복사 (자세한 내용이 있으면 같이) */
	async function copyOne(e: LogEntry) {
		const d = detail(e);
		try {
			await navigator.clipboard.writeText(`[${new Date(e.time).toISOString()}] ${e.level.toUpperCase()} ${e.source}: ${e.message}${d ? `
${d}` : ""}`);
			toasts.success($_("settings.more.copiedToast"));
		} catch {
			toasts.error($_("settings.more.copyFailToast"));
		}
	}
	// 지난 실행까지의 백엔드 panic 기록 (앱 데이터 폴더 logs/panic.log). 이 화면을 열 때만 읽는다
	let panics = $state<PanicEntry[]>([]);
	onMount(async () => {
		panics = await call(commands.getPanicLog(), { source: "log", silent: true }).catch(() => []);
	});
	const when = (t: number) => {
		const d = new Date(t);
		return `${d.getMonth() + 1}/${d.getDate()} ${clock(t)}`;
	};
	async function clearPanics() {
		if (!(await confirmAction({ title: $_("settings.more.clearPanicTitle"), message: $_("settings.more.clearPanicMsg"), confirmLabel: $_("common.delete"), danger: true }))) return;
		await call(commands.clearPanicLog(), { source: "log" });
		panics = [];
	}

	async function copyAll() {
		const text = shown
			.map((e) => `[${new Date(e.time).toISOString()}] ${e.level.toUpperCase()} ${e.source}: ${e.message}${detail(e) ? `\n${detail(e)}` : ""}`)
			.join("\n");
		try {
			await navigator.clipboard.writeText(text);
			toasts.success($_("settings.more.copiedToast"));
		} catch {
			toasts.error($_("settings.more.copyFailToast"));
		}
	}
</script>

<Section title={$_("settings.logSec.title")} lead={$_("settings.logSec.lead")}>
	{#snippet actions()}
		<Button size="sm" disabled={!shown.length} onclick={copyAll}>{#snippet icon()}<Copy />{/snippet}{$_("common.copy")}</Button>
		<Button size="sm" variant="ghost" disabled={!logs.entries.length} onclick={() => logs.clear()}>{#snippet icon()}<Trash2 />{/snippet}{$_("settings.logSec.clear")}</Button>
	{/snippet}
	<div class="filter">
		<Segmented
			label={$_("settings.logSec.title")}
			bind:value={level}
			options={[
				{ value: "all", label: `${$_("common.all")} ${logs.entries.length}` },
				{ value: "error", label: `${$_("settings.logSec.errors")} ${logs.errorCount}` },
				{ value: "warn", label: $_("settings.logSec.warns") },
				{ value: "info", label: $_("settings.logSec.infos") },
			]}
		/>
	</div>
	<Group flush>
		{#each shown as l (l.id)}
			{@const d = detail(l)}
			<div class="row">
				<div class="line-wrap">
					<button class="line" title={$_("common.copy")} onclick={() => copyOne(l)}>
						<Badge tone={tone(l.level)}>{l.level}</Badge>
						<span class="t">{clock(l.time)}</span>
						<span class="src">{l.source}</span>
						<span class="m">{l.message}</span>
					</button>
					{#if d}<button class="more" aria-expanded={open === l.id} onclick={() => (open = open === l.id ? null : l.id)}>{open === l.id ? $_("settings.logSec.collapse") : $_("settings.logSec.details")}</button>{/if}
				</div>
				{#if d && open === l.id}<pre>{d}</pre>{/if}
			</div>
		{:else}
			<EmptyState title={$_("settings.logSec.emptyTitle")} description={$_("settings.logSec.emptyDesc")} />
		{/each}
	</Group>

	<SyncRunsGroup />

	<Group title={$_("settings.logSec.panicGroup")} flush>
		{#snippet aside()}
			<Button size="sm" variant="ghost" disabled={!panics.length} onclick={clearPanics}>{#snippet icon()}<Trash2 />{/snippet}{$_("common.delete")}</Button>
		{/snippet}
		{#each panics as p, i (i)}
			<div class="row">
				<button class="line" title={$_("common.copy")} onclick={() => copyOne({ id: -1, level: "error", time: p.time, source: "panic", message: p.message, details: { location: p.location, thread: p.thread } })}>
					<Badge tone="danger">panic</Badge>
					<span class="t">{when(p.time)}</span>
					<span class="src">{p.thread}</span>
					<span class="m">{p.message}{p.location ? ` (${p.location})` : ""}</span>
				</button>
			</div>
		{:else}
			<p class="none">{$_("settings.logSec.noPanics")}</p>
		{/each}
	</Group>
</Section>

<style>
	.none {
		margin: 0;
		padding: var(--space-4) var(--space-2);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.filter {
		margin-bottom: var(--space-3);
	}
	.row + .row {
		border-top: 1px solid var(--border);
	}
	.line {
		display: grid;
		grid-template-columns: 56px 64px 72px 1fr;
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
	}
	.line-wrap {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}
	.line {
		flex: 1;
		cursor: copy;
	}
	.line:hover {
		background: var(--surface-hover);
	}
	.more {
		flex-shrink: 0;
		height: 24px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.more:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.t,
	.src {
		color: var(--text-faint);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}
	.m {
		overflow-wrap: anywhere;
	}
	pre {
		margin: 0 var(--space-2) var(--space-2);
		padding: var(--space-3);
		border-radius: var(--radius-sm);
		background: var(--surface);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		max-height: 240px;
		overflow: auto;
	}
</style>
