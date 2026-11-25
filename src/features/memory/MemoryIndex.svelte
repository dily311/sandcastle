<script lang="ts">
	import { _ } from "svelte-i18n";
	// 색인 버튼: 먼저 미리보기(요금 없음)를 보여 주고, 확인했을 때만 색인을 돌린다 (ADR-015). 상한을 넘으면 한 번 더 확인
	import type { IndexPreview } from "$ipc/bindings";
	import { Button, Dialog } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";

	let { conversationId = null, onDone }: { conversationId?: string | null; onDone?: () => void } = $props();

	const src = { source: "memory" };
	const scope = $derived({ conversationId });
	let open = $state(false);
	let pv = $state<IndexPreview | null>(null);
	let loading = $state(false);
	let running = $state(false);
	let overOk = $state(false);

	async function showPreview() {
		loading = true;
		try {
			pv = await call(commands.previewMemoryIndex(scope), src);
			overOk = false;
			open = true;
		} catch {
			/* 토스트는 call이 띄움 */
		} finally {
			loading = false;
		}
	}
	async function run() {
		if (!pv) return;
		running = true;
		try {
			const r = await call(commands.runMemoryIndex(scope, overOk), src);
			toasts.success($_("extraTools.memIndexDone", { values: { embedded: r.embedded, cached: r.cached } }));
			open = false;
		} catch {
			/* 실패해도 쓴 배치는 남음 — 상태에서 확인 */
		} finally {
			running = false;
			onDone?.();
		}
	}
	const nothing = $derived(!!pv && pv.toEmbed === 0);
</script>

<Button {loading} onclick={showPreview}>{conversationId ? $_("extraTools.memIndexConvBtn") : $_("extraTools.memIndexAllBtn")}</Button>

<Dialog bind:open title={$_("extraTools.memIndexTitle")} description={$_("extraTools.memIndexDesc")}>
	{#if pv}
		<ul class="pv">
			<li>{$_("extraTools.memIndexStat1", { values: { convs: pv.conversations, chunks: pv.chunks } })}</li>
			<li>{$_("extraTools.memIndexStat2", { values: { cached: pv.cached } })}</li>
			<li><b>{$_("extraTools.memIndexStat3", { values: { toEmbed: pv.toEmbed } })}</b> {$_("extraTools.memIndexStat3Est", { values: { tokens: pv.estTokens.toLocaleString() } })} <span class="faint">{$_("extraTools.memIndexEst")}</span></li>
		</ul>
		<p class="faint">{$_("extraTools.memIndexNotice")}</p>
		{#if pv.overCap}
			<label class="over">
				<input type="checkbox" bind:checked={overOk} />
				<span>{$_("extraTools.memIndexOverCap", { values: { cap: pv.tokenCap.toLocaleString() } })}</span>
			</label>
		{/if}
	{/if}
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (open = false)}>{$_("common.cancel")}</Button>
		<Button variant="primary" loading={running} disabled={nothing || (!!pv && pv.overCap && !overOk)} onclick={run}>{nothing ? $_("extraTools.memIndexNothing") : $_("extraTools.memIndexStart")}</Button>
	{/snippet}
</Dialog>

<style>
	.pv {
		margin: 0 0 var(--space-3);
		padding-left: var(--space-4);
		font-size: var(--text-sm);
	}
	.faint {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	p.faint {
		margin: 0 0 var(--space-3);
	}
	.over {
		display: flex;
		gap: var(--space-2);
		align-items: flex-start;
		font-size: var(--text-sm);
		color: var(--warning);
	}
</style>
