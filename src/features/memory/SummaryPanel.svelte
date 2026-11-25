<script lang="ts">
	import { _ } from "svelte-i18n";
	// 대화 요약 (3-D, ADR-016): 오른쪽 패널 "이 대화"의 한 구역. 요약 글은 AI에게 가고(고쳐도 그대로), 메모는 가지 않는다
	import { onMount } from "svelte";
	import type { SummaryInfo } from "$ipc/bindings";
	import { Button, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import type { Conversation } from "$features/character/library.svelte";

	let { conv, memo = "" }: { conv: Conversation; memo?: string } = $props();

	const src = { source: "summary" };
	let info = $state<SummaryInfo | null>(null);
	let text = $state("");
	let busy = $state(false);

	function apply(i: SummaryInfo | null) {
		info = i;
		text = i?.text ?? "";
	}
	async function load() {
		apply(await call(commands.getConversationSummary(conv.id), { ...src, silent: true }).catch(() => null));
	}
	onMount(load);
	$effect(() => {
		void conv.id;
		void load();
	});

	async function save() {
		if (text.trim() === (info?.text ?? "").trim()) return;
		apply(await call(commands.setConversationSummary(conv.id, text), src));
	}
	async function now() {
		busy = true;
		try {
			apply(await call(commands.summarizeConversationNow(conv.id), src));
			toasts.success($_("extraTools.summaryDone"));
		} catch {
			/* 토스트는 call이 띄움 */
		} finally {
			busy = false;
		}
	}
	async function undo() {
		const r = await call(commands.undoConversationSummary(conv.id), src);
		if (r) apply(r);
	}
	async function clear() {
		if (!(await confirmAction({ title: $_("extraTools.summaryClearTitle"), message: $_("extraTools.summaryClearMsg"), confirmLabel: $_("extraTools.summaryClearBtn") }))) return;
		await call(commands.clearConversationSummary(conv.id), src);
		apply(null);
	}
	async function addMemo() {
		apply(await call(commands.setConversationSummary(conv.id, `${memo.trim()}\n${text}`.trim()), src));
	}
	const reason = $derived<Record<string, string>>({
		budget: $_("extraTools.summaryReason.budget"),
		timeout: $_("extraTools.summaryReason.timeout"),
		cancelled: $_("extraTools.summaryReason.cancelled"),
		error: $_("extraTools.summaryReason.error"),
		empty: $_("extraTools.summaryReason.empty"),
	});
</script>

<section>
	<h4>{$_("extraTools.summaryHeading")}</h4>
	<Textarea bind:value={text} autosize rows={4} maxRows={14} placeholder={$_("extraTools.summaryPlaceholder")} aria-label={$_("extraTools.summaryHeading")} onblur={save} />
	<div class="bar">
		{#if info?.text}<span class="stat">{$_("extraTools.summaryStat", { values: { count: info.coveredCount, tokens: info.tokens } })}</span>{/if}
		{#if info?.failure}<span class="stat warn">{$_("extraTools.summaryAutoFail", { values: { reason: reason[info.failure.reason] ?? $_("extraTools.summaryReason.error") } })}</span>{/if}
		<span class="grow"></span>
		{#if memo.trim()}<Button size="sm" variant="ghost" onclick={addMemo}>{$_("extraTools.summaryAppendMemo")}</Button>{/if}
		{#if info?.hasPrev}<Button size="sm" variant="ghost" onclick={undo}>{$_("extraTools.revertBtn")}</Button>{/if}
		{#if info}<Button size="sm" variant="ghost" onclick={clear}>{$_("extraTools.summaryClearBtn")}</Button>{/if}
		<Button size="sm" loading={busy} onclick={now}>{$_("extraTools.summaryNowBtn")}</Button>
	</div>
</section>

<style>
	.bar {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-top: var(--space-2);
	}
	.grow {
		flex: 1;
	}
	.stat {
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.stat.warn {
		color: var(--warning);
	}
</style>
