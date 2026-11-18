<script lang="ts">
	// 메시지 아래: 플러그인이 고친 표시와 메모 (6-D-4). 원문 보기 / 되돌리기.
	// 코어 플러그인을 꺼도 보인다 — 고친 것을 되돌릴 수 있어야 하므로 (되돌리기 커맨드는 꺼져 있어도 됨)
	import type { Message_Serialize as Message } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { pluginHost } from "./host.svelte";
	import { pluginProcessOf } from "./postprocess";

	let {
		msg,
		showOriginal = $bindable(false),
		onupdate,
	}: { msg: Message; showOriginal?: boolean; onupdate?: (m: Message) => void } = $props();

	const pp = $derived(pluginProcessOf(msg));
	/** 플러그인이 고친 글이 그대로일 때만 (그 뒤 사용자가 고쳤으면 원문 보기·되돌리기를 숨김) */
	const fixed = $derived(!!pp && pp.changedBy.length > 0 && pp.content === msg.content);
	const notes = $derived(pp ? Object.entries(pp.notes ?? {}).flatMap(([id, list]) => list.map((n) => ({ id, ...n }))) : []);
	let busy = $state(false);

	const nameOf = (id: string) => pluginHost.active.find((p) => p.id === id)?.name ?? id;

	async function revert() {
		busy = true;
		try {
			const m = await call(commands.revertPluginPostprocess(msg.id), { source: "plugins" });
			showOriginal = false;
			onupdate?.(m);
		} catch {
			/* 토스트 */
		} finally {
			busy = false;
		}
	}
</script>

{#if pp && (fixed || notes.length)}
	<div class="pp">
		{#if fixed}
			<p class="line">
				{showOriginal ? "원문" : `${pp.changedBy.map(nameOf).join(", ")}이(가) 고침`} ·
				<button class="link" onclick={() => (showOriginal = !showOriginal)}>{showOriginal ? "고친 글 보기" : "원문 보기"}</button> ·
				<button class="link" disabled={busy} onclick={revert}>되돌리기</button>
			</p>
		{/if}
		{#if notes.length}
			<ul class="notes">
				{#each notes as n, i (i)}
					<li class:warn={n.level === "warn"}><span class="who">{nameOf(n.id)}</span> {n.text}</li>
				{/each}
			</ul>
		{/if}
	</div>
{/if}

<style>
	.pp {
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
	.who {
		color: var(--text-muted);
		font-weight: 600;
	}
</style>
