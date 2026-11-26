<script lang="ts">
	import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import { toasts } from "$shared/stores/toast.svelte";

	const icons = { info: Info, success: CircleCheck, warning: TriangleAlert, error: CircleAlert };
</script>

<div class="stack" aria-live="polite" aria-relevant="additions">
	{#each toasts.items as t (t.id)}
		{@const Icon = icons[t.tone]}
		<div class="toast {t.tone}" role={t.tone === "error" ? "alert" : "status"}>
			<Icon class="ic" />
			<div class="txt">
				<p>{t.message}</p>
				{#if t.detail}<small>{t.detail}</small>{/if}
			</div>
			{#if t.action}
				<button class="act" onclick={() => { t.action?.run(); toasts.dismiss(t.id); }}>{t.action.label}</button>
			{/if}
			<button class="x" aria-label={$_("toast.close")} onclick={() => toasts.dismiss(t.id)}><X /></button>
		</div>
	{/each}
</div>

<style>
	.stack {
		position: fixed;
		right: var(--space-4);
		/* 휴대폰: 하단 탐색 막대 위로 */
		bottom: calc(var(--space-4) + var(--safe-bottom));
		z-index: var(--z-toast);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		width: min(380px, calc(100vw - 32px));
		pointer-events: none;
	}
	.toast {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		padding: var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: var(--shadow-lg);
		pointer-events: auto;
		animation: in var(--duration) var(--ease);
	}
	@keyframes in {
		from {
			opacity: 0;
			transform: translateY(6px);
		}
	}
	.toast :global(.ic) {
		width: 18px;
		height: 18px;
		flex: none;
		margin-top: 1px;
	}
	.info :global(.ic) {
		color: var(--info);
	}
	.success :global(.ic) {
		color: var(--success);
	}
	.warning :global(.ic) {
		color: var(--warning);
	}
	.error {
		border-color: color-mix(in oklab, var(--danger) 35%, var(--border));
	}
	.error :global(.ic) {
		color: var(--danger);
	}
	.txt {
		flex: 1;
		min-width: 0;
		font-size: var(--text-sm);
	}
	small {
		color: var(--text-faint);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}
	.act {
		border: none;
		background: none;
		color: var(--accent-text);
		font-weight: 600;
		font-size: var(--text-sm);
		cursor: pointer;
	}
	.x {
		display: grid;
		place-items: center;
		padding: 2px;
		border: none;
		background: none;
		color: var(--text-faint);
		cursor: pointer;
	}
	.x :global(svg) {
		width: 14px;
		height: 14px;
	}
</style>
