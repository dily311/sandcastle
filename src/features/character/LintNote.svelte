<script lang="ts">
	// 메시지 아래: linter가 걸러 낸 것과 고친 것 (3-B-3). 원문 보기 / 되돌리기.
	// 코어 플러그인을 꺼도 보인다 — 고친 것을 되돌릴 수 있어야 하므로
	import { _ } from "svelte-i18n";
	import type { Message_Serialize as Message } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { lintOf } from "./lint";

	let { msg, showOriginal = $bindable(false), onupdate }: { msg: Message; showOriginal?: boolean; onupdate?: (m: Message) => void } = $props();

	const lint = $derived(lintOf(msg));
	type Note = { status: string; detail?: { reason?: string; attempts?: number; attempt?: number } };
	const meta = $derived((msg.meta ?? {}) as { rewriteNote?: Note; info?: { rewrite?: { attempt: number; max: number } } });
	const rw = $derived(meta.info?.rewrite);
	const note = $derived(meta.rewriteNote);
	const REASON = $derived<Record<string, string>>({ BudgetExceeded: $_("chatExtra.lintNote.reasonBudget"), AiProviderTimeout: $_("chatExtra.lintNote.reasonTimeout") });
	const fixed = $derived(!!lint?.original);
	let busy = $state(false);

	async function revert() {
		busy = true;
		try {
			const m = await call(commands.revertLint(msg.id), { source: "lint" });
			showOriginal = false;
			onupdate?.(m);
		} catch {
			/* 토스트 */
		} finally {
			busy = false;
		}
	}
</script>

{#if rw || note}
	<div class="lint">
		{#if rw}<p class="line">{$_("chatExtra.lintNote.rewriteAttempt", { values: { attempt: rw.attempt, max: rw.max } })}</p>{/if}
		{#if note?.status === "exhausted"}<p class="line">{$_("chatExtra.lintNote.rewriteExhausted", { values: { attempts: note.detail?.attempts ?? 0 } })}</p>{/if}
		{#if note?.status === "failed"}<p class="line warn">{$_("chatExtra.lintNote.rewriteFailed", { values: { reason: REASON[note.detail?.reason ?? ""] ?? note.detail?.reason ?? "" } })}</p>{/if}
	</div>
{/if}
{#if lint}
	<div class="lint">
		{#if fixed}
			<p class="line">
				{showOriginal ? $_("chatExtra.lintNote.original") : $_("chatExtra.lintNote.fixedByLinter")} ·
				<button class="link" onclick={() => (showOriginal = !showOriginal)}>{showOriginal ? $_("chatExtra.lintNote.showFixed") : $_("chatExtra.lintNote.showOriginal")}</button> ·
				<button class="link" disabled={busy} onclick={revert}>{$_("chatExtra.lintNote.revert")}</button>
			</p>
		{/if}
		<ul class="notes">
			{#each lint.findings as f, i (i)}
				<li class:warn={f.level === "warn"}>{f.text}{f.fixed ? $_("chatExtra.lintNote.fixedSuffix") : ""}</li>
			{/each}
		</ul>
	</div>
{/if}

<style>
	.lint {
		margin: 0 0 var(--space-1);
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.line {
		margin: 0;
	}
	.line.warn {
		color: var(--warning);
	}
	.link {
		padding: 0;
		border: none;
		background: none;
		color: var(--accent-text);
		font: inherit;
		cursor: pointer;
	}
	.link:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.notes {
		margin: var(--space-1) 0 0;
		padding-left: var(--space-4);
	}
	.notes li.warn {
		color: var(--warning);
	}
</style>
