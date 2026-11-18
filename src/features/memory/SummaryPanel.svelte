<script lang="ts">
	// 대화 요약 (3-D, ADR-016): 오른쪽 패널 "이 대화"의 한 구역. 요약 글은 AI에게 가고(고쳐도 그대로), 메모는 가지 않는다
	import { onMount } from "svelte";
	import type { SummaryInfo } from "$ipc/bindings";
	import { Button, Switch, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { library, type Conversation } from "$features/character/library.svelte";

	let { conv, memo = "" }: { conv: Conversation; memo?: string } = $props();

	const src = { source: "summary" };
	const off = $derived((conv.meta as { summaryOff?: boolean } | null)?.summaryOff === true);
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
			toasts.success("요약함");
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
		if (!(await confirmAction({ title: "요약 지우기", message: "요약을 지우면 다음 자동 요약이 처음부터 다시 이어 씀 (요금이 듦)", confirmLabel: "지우기" }))) return;
		await call(commands.clearConversationSummary(conv.id), src);
		apply(null);
	}
	async function addMemo() {
		apply(await call(commands.setConversationSummary(conv.id, `${memo.trim()}\n${text}`.trim()), src));
	}
	const reason: Record<string, string> = { budget: "예산 한도", timeout: "시간 초과", cancelled: "취소됨", error: "실패", empty: "빈 응답" };
</script>

<section>
	<h4>요약</h4>
	<Switch checked={!off} label="이 대화에 요약 쓰기" onchange={(v) => library.patchMeta(conv.id, { summaryOff: v ? null : true })} />
	{#if !off}
		<Textarea bind:value={text} autosize rows={4} maxRows={14} placeholder="아직 요약 없음" aria-label="요약" onblur={save} />
		<div class="bar">
			{#if info?.text}<span class="stat">{info.coveredCount}개 반영 · {info.tokens}토큰</span>{/if}
			{#if info?.failure}<span class="stat warn">자동 요약 {reason[info.failure.reason] ?? "실패"}</span>{/if}
			<span class="grow"></span>
			{#if memo.trim()}<Button size="sm" variant="ghost" onclick={addMemo}>메모 붙이기</Button>{/if}
			{#if info?.hasPrev}<Button size="sm" variant="ghost" onclick={undo}>되돌리기</Button>{/if}
			{#if info}<Button size="sm" variant="ghost" onclick={clear}>지우기</Button>{/if}
			<Button size="sm" loading={busy} onclick={now}>지금 요약</Button>
		</div>
	{/if}
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
