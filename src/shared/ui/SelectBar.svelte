<script lang="ts">
	// 여러 개 고르기 줄: 모두 선택 · (추가 동작) · N개 삭제
	import type { Snippet } from "svelte";
	import { _ } from "svelte-i18n";
	import { Trash2 } from "@lucide/svelte";
	import Button from "./Button.svelte";

	interface Props {
		count: number;
		/** "개" / "장" */
		unit?: string;
		onall: () => void;
		ondelete: () => void;
		/** 삭제 옆 다른 동작 (예: 태그 달기) */
		children?: Snippet;
	}
	let { count, unit, onall, ondelete, children }: Props = $props();
	const effectiveUnit = $derived(unit ?? $_("common.unitItem"));
</script>

<div class="selbar" role="toolbar" aria-label={$_("sharedControls.selectToolbarAria")}>
	<Button size="sm" variant="ghost" onclick={onall}>{$_("common.selectAll")}</Button>
	<span class="picked-count">{$_("common.selectedCount", { values: { count, unit: effectiveUnit } })}</span>
	<span class="sp"></span>
	{#if children}{@render children()}{/if}
	<Button size="sm" variant="danger" disabled={!count} onclick={ondelete}>{#snippet icon()}<Trash2 />{/snippet}{$_("common.deleteCount", { values: { count, unit: effectiveUnit } })}</Button>
</div>

<style>
	/* 폭은 그 화면의 목록 폭을 따른다 (--selectbar-max, 없으면 꽉 채움) */
	.selbar {
		width: 100%;
		max-width: var(--selectbar-max, none);
		margin: 0 auto var(--space-3);
		box-sizing: border-box;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border: 1px solid color-mix(in oklab, var(--accent) 32%, var(--border));
		border-radius: var(--radius-md);
		background: var(--accent-soft);
		font-size: var(--text-sm);
	}
	.picked-count {
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--accent-text);
		font-variant-numeric: tabular-nums;
	}
	.sp {
		flex: 1;
	}
</style>
