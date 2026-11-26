<script lang="ts">
	import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import { toasts } from "$shared/stores/toast.svelte";

	const icons = { info: Info, success: CircleCheck, warning: TriangleAlert, error: CircleAlert };
</script>

<div class="stack" aria-live="polite" aria-relevant="additions">
	{#each toasts.items as t (t.id)}
		{@const Icon = icons[t.tone]}
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<div
			class="toast {t.tone}"
			class:clickable={t.dismissOnClick}
			role={t.tone === "error" ? "alert" : "status"}
			tabindex={t.dismissOnClick ? 0 : undefined}
			onclick={() => {
				if (t.dismissOnClick) toasts.dismiss(t.id, true);
			}}
			onkeydown={(e) => {
				if (t.dismissOnClick && (e.key === "Enter" || e.key === " ")) {
					e.preventDefault();
					toasts.dismiss(t.id, true);
				}
			}}
		>
			<Icon class="ic" />
			<div class="txt">
				<p>{t.message}</p>
				{#if t.progress}
					{#if !t.progress.indeterminate && typeof t.progress.total === "number" && t.progress.total > 0}
						<progress class="prog" max={t.progress.total} value={t.progress.current ?? 0}></progress>
					{:else}
						<progress class="prog"></progress>
					{/if}
				{/if}
				{#if t.detail}<small>{t.detail}</small>{/if}
			</div>
			{#if t.action}
				<button
					class="act"
					onclick={(e) => {
						e.stopPropagation();
						t.action?.run();
						toasts.dismiss(t.id, false);
					}}>{t.action.label}</button
				>
			{/if}
			<button
				class="x"
				aria-label={$_("toast.close")}
				onclick={(e) => {
					e.stopPropagation();
					toasts.dismiss(t.id, true);
				}}><X /></button
			>
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
	.toast.clickable {
		cursor: pointer;
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
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: var(--text-sm);
	}
	.txt p {
		margin: 0;
	}
	.prog {
		width: 100%;
		height: 5px;
		accent-color: var(--accent);
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
