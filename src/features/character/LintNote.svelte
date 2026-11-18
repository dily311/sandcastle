<script lang="ts">
	// 메시지 아래: linter가 걸러 낸 것과 고친 것 (3-B-3). 원문 보기 / 되돌리기.
	// 코어 플러그인을 꺼도 보인다 — 고친 것을 되돌릴 수 있어야 하므로
	import type { Message_Serialize as Message } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { lintOf } from "./lint";

	let { msg, showOriginal = $bindable(false), onupdate }: { msg: Message; showOriginal?: boolean; onupdate?: (m: Message) => void } = $props();

	const lint = $derived(lintOf(msg));
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

{#if lint}
	<div class="lint">
		{#if fixed}
			<p class="line">
				{showOriginal ? "원문" : "linter가 고침"} ·
				<button class="link" onclick={() => (showOriginal = !showOriginal)}>{showOriginal ? "고친 글 보기" : "원문 보기"}</button> ·
				<button class="link" disabled={busy} onclick={revert}>되돌리기</button>
			</p>
		{/if}
		<ul class="notes">
			{#each lint.findings as f, i (i)}
				<li class:warn={f.level === "warn"}>{f.text}{f.fixed ? " → 고침" : ""}</li>
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
