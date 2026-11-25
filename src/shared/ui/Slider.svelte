<script lang="ts">
	import { RotateCcw } from "@lucide/svelte";

	// 숫자 설정용 슬라이더 + 정확한 값 입력칸.
	// value가 빈 문자열이면 "정하지 않음"(공급자/앱 기본값)이고, 슬라이더는 흐리게 fallback 위치에 있음
	interface Props {
		label: string;
		hint?: string;
		value: string;
		min: number;
		max: number;
		step: number;
		/** 비었을 때 슬라이더 위치 */
		fallback: number;
		/** 비었을 때 보이는 말 */
		unsetLabel?: string;
		/** 값 표시 (예: 1024 → "1,024") */
		format?: (n: number) => string;
		/** 놓거나 입력·초기화로 값이 확정될 때 */
		oncommit?: (v: string) => void;
	}
	let { label, hint, value = $bindable(""), min, max, step, fallback, unsetLabel = "기본값", format = (n) => String(n), oncommit }: Props = $props();

	const id = `sl-${Math.random().toString(36).slice(2, 8)}`;
	const unset = $derived(String(value).trim() === "" || Number.isNaN(Number(value)));
	const num = $derived(unset ? fallback : Number(value));
	const pct = $derived(((Math.min(max, Math.max(min, num)) - min) / (max - min)) * 100);
</script>

<div class="slider" class:unset>
	<div class="top">
		<label for={id}>{label}</label>
		{#if !unset && format(num) !== String(num)}<span class="val">{format(num)}</span>{/if}
		<input
			class="exact"
			type="number"
			{min}
			{max}
			{step}
			value={unset ? "" : value}
			placeholder={unsetLabel}
			aria-label="{label} 직접 입력"
			onchange={(e) => {
				value = e.currentTarget.value.trim();
				oncommit?.(value);
			}}
		/>
		<button
			type="button"
			class="reset"
			title="{unsetLabel}(으)로"
			aria-label="{label} {unsetLabel}(으)로"
			disabled={unset}
			onclick={() => {
				value = "";
				oncommit?.("");
			}}><RotateCcw /></button
		>
	</div>
	<input
		{id}
		type="range"
		{min}
		{max}
		{step}
		value={num}
		style:--pct="{pct}%"
		oninput={(e) => (value = e.currentTarget.value)}
		onchange={(e) => oncommit?.(e.currentTarget.value)}
	/>
	{#if hint}<p class="hint">{hint}</p>{/if}
</div>

<style>
	.slider {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.top {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	label {
		flex: 1;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.val {
		font-size: var(--text-sm);
		font-variant-numeric: tabular-nums;
		color: var(--text);
		opacity: 0;
	}
	.unset .val {
		color: var(--text-faint);
	}
	.exact {
		width: 84px;
		height: 26px;
		padding: 0 6px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-xs);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: var(--text-xs);
		font-variant-numeric: tabular-nums;
	}
	.exact:focus {
		outline: 2px solid var(--focus-ring);
		border-color: var(--accent);
	}
	.reset {
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		padding: 0;
		border: none;
		border-radius: var(--radius-xs);
		background: none;
		color: var(--text-muted);
		cursor: pointer;
	}
	.reset:disabled {
		opacity: 0.35;
		cursor: default;
	}
	.reset:not(:disabled):hover {
		background: var(--surface-hover);
	}
	.reset :global(svg) {
		width: 14px;
		height: 14px;
	}
	input[type="range"] {
		appearance: none;
		width: 100%;
		height: 6px;
		margin: 6px 0;
		border-radius: 999px;
		background: linear-gradient(to right, var(--accent) var(--pct), var(--surface-active) var(--pct));
		cursor: pointer;
	}
	.unset input[type="range"] {
		background: var(--surface-active);
	}
	input[type="range"]::-webkit-slider-thumb {
		appearance: none;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--surface);
		border: 2px solid var(--accent);
		box-shadow: var(--shadow-sm);
	}
	.unset input[type="range"]::-webkit-slider-thumb {
		border-color: var(--border-strong);
	}
	input[type="range"]:focus-visible {
		outline: 2px solid var(--focus-ring);
		outline-offset: 4px;
	}
	.hint {
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
</style>
