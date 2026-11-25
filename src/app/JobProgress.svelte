<script lang="ts">
	// 파일 작업 대기열의 진행 (5-E-28): 화면 아래, 둘 이상이거나 오래 걸릴 때만.
	// 지금 것·남은 수, 펼치면 기다리는 목록(하나씩 빼기·모두 빼기), 멈출 수 있는 작업은 멈추기
	import { _ } from "svelte-i18n";
	import { ChevronDown, ListX, Square, X } from "@lucide/svelte";
	import { IconButton } from "$shared/ui";
	import { jobs } from "$shared/stores/jobs.svelte";

	/** 화면 상태: 기다리는 목록 펼침 */
	let open = $state(false);
	$effect(() => {
		if (!jobs.waiting.length) open = false;
	});
</script>

{#if jobs.visible && jobs.current}
	<div class="jobs sc-job-progress" role="status" aria-live="polite">
		<div class="text">
			<b>{jobs.current.group} {jobs.done + 1}/{jobs.total}</b>
			<span class="label">{jobs.stopping ? $_("settings.mascot.jobStopping") : jobs.current.label}</span>
		</div>
		<div class="tools">
			{#if jobs.waiting.length}
				<IconButton label={open ? $_("settings.mascot.jobCollapseWaiting") : $_("settings.mascot.jobShowWaiting", { values: { count: jobs.waiting.length } })} size="sm" aria-expanded={open} onclick={() => (open = !open)}><ChevronDown class={open ? "" : "rot"} /></IconButton>
			{/if}
			{#if jobs.current.stoppable}
				<IconButton label={$_("settings.mascot.jobStopCurrent")} size="sm" disabled={jobs.stopping} onclick={() => jobs.stop()}><Square /></IconButton>
			{:else if jobs.waiting.length}
				<IconButton label={$_("settings.mascot.jobClearWaitingAria")} size="sm" onclick={() => jobs.clear()}><ListX /></IconButton>
			{/if}
		</div>
		<progress max={jobs.total} value={jobs.done}></progress>
		{#if open}
			<ul class="waiting" aria-label={$_("settings.mascot.jobWaitingAria")}>
				{#each jobs.waiting as w (w.id)}
					<li>
						<span>{w.label}</span>
						<IconButton label={$_("settings.mascot.jobRemoveItem", { values: { label: w.label } })} size="sm" onclick={() => jobs.remove(w.id)}><X /></IconButton>
					</li>
				{/each}
			</ul>
			<button type="button" class="clear" onclick={() => jobs.clear()}>{$_("settings.mascot.jobClearAll")}</button>
		{/if}
	</div>
{/if}

<style>
	.jobs {
		position: fixed;
		left: 50%;
		bottom: calc(var(--space-6) + env(safe-area-inset-bottom, 0px));
		transform: translateX(-50%);
		z-index: var(--z-toast);
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: var(--space-1) var(--space-3);
		width: min(420px, calc(100vw - var(--space-6)));
		padding: var(--space-3) var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: var(--shadow-md);
	}
	.text {
		display: flex;
		flex-direction: column;
		min-width: 0;
		font-size: var(--text-sm);
	}
	.label {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--text-muted);
		font-size: var(--text-xs);
	}
	.tools {
		display: flex;
		gap: 2px;
	}
	.tools :global(.rot) {
		transform: rotate(180deg);
	}
	progress {
		grid-column: 1 / -1;
		width: 100%;
		height: 6px;
		accent-color: var(--accent);
	}
	.waiting {
		grid-column: 1 / -1;
		max-height: 30vh;
		overflow-y: auto;
		margin: 0;
		padding: 0;
		list-style: none;
		font-size: var(--text-xs);
	}
	.waiting li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
		color: var(--text-muted);
	}
	.waiting span {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.clear {
		grid-column: 1 / -1;
		justify-self: end;
		border: none;
		background: none;
		color: var(--text-muted);
		font-size: var(--text-xs);
		cursor: pointer;
	}
</style>
