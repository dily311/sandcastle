<script lang="ts">
	import { Package } from "@lucide/svelte";
	import type { ItemGainNotice } from "./playUtils";
	let { notices }: { notices: ItemGainNotice[] } = $props();
</script>

{#if notices.length}
	<div class="item-gain-stack" aria-live="polite" aria-label="아이템 획득 알림">
		{#each notices as notice (notice.id)}
			<div class="item-gain-card">
				<div class="item-gain-icon"><Package size={19} /></div>
				<div class="item-gain-copy"><small>아이템 획득</small><strong>{notice.name}</strong></div>
				<span class="item-gain-quantity">× {notice.quantity}</span>
			</div>
		{/each}
	</div>
{/if}

<style>
	.item-gain-stack {
		position: absolute;
		z-index: 110;
		top: 18px;
		left: 50%;
		display: grid;
		gap: 8px;
		width: min(340px, calc(100% - 32px));
		transform: translateX(-50%);
		pointer-events: none;
	}
	.item-gain-card {
		display: flex;
		align-items: center;
		gap: 11px;
		padding: 12px 14px;
		border: 1px solid var(--game-accent-border);
		border-radius: var(--radius-lg);
		color: var(--game-text-primary);
		background: var(--game-panel-bg);
		box-shadow: var(--game-shadow-panel);
		backdrop-filter: blur(14px);
		animation: item-gain-enter 0.28s ease-out both;
	}
	.item-gain-icon {
		display: grid;
		place-items: center;
		width: 36px;
		height: 36px;
		flex: 0 0 36px;
		border-radius: var(--radius-md);
		color: var(--game-accent);
		background: var(--game-accent-soft);
	}
	.item-gain-copy {
		display: grid;
		flex: 1;
		min-width: 0;
		gap: 3px;
	}
	.item-gain-copy small {
		color: var(--game-accent);
		font-size: 10px;
		letter-spacing: 0.04em;
	}
	.item-gain-copy strong {
		overflow: hidden;
		font-size: 13px;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.item-gain-quantity {
		flex: 0 0 auto;
		color: var(--game-accent-strong);
		font-size: 13px;
		font-weight: 700;
	}
	@keyframes item-gain-enter {
		from {
			opacity: 0;
			transform: translateY(-8px) scale(0.98);
		}
		to {
			opacity: 1;
			transform: translateY(0) scale(1);
		}
	}
</style>
