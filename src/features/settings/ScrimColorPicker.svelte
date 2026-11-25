<script lang="ts">
	import { theme } from "$app/theme.svelte";

	const PRESETS: { value: string; label: string }[] = [
		{ value: "#000000", label: "검정" },
		{ value: "#ffffff", label: "흰색" },
		{ value: "#0f172a", label: "네이비" },
		{ value: "#1c102b", label: "딥 퍼플" },
		{ value: "#221611", label: "웜 브라운" },
		{ value: "#0f2318", label: "딥 그린" },
	];

	const cur = $derived(theme.scrimColor);
	const isCustom = $derived(!!cur && !PRESETS.some((p) => p.value.toLowerCase() === cur.toLowerCase()));
	const pickerValue = $derived(cur ?? (theme.dark ? "#000000" : "#ffffff"));
</script>

<div class="scrim-colors" role="radiogroup" aria-label="가림막 색상">
	<button
		type="button"
		role="radio"
		class="auto"
		class:on={cur === null}
		aria-checked={cur === null}
		title="현재 테마 바탕색 자동 적용"
		onclick={() => theme.setScrimColor(null)}
	>
		테마 기본
	</button>
	{#each PRESETS as p (p.value)}
		<button
			type="button"
			role="radio"
			class="dot"
			class:on={cur?.toLowerCase() === p.value}
			aria-checked={cur?.toLowerCase() === p.value}
			aria-label={p.label}
			title={p.label}
			style:background={p.value}
			onclick={() => theme.setScrimColor(p.value)}
		></button>
	{/each}
	<input
		type="color"
		class="custom"
		class:on={isCustom}
		value={pickerValue}
		aria-label="가림막 색상 직접 고르기"
		title="직접 고르기"
		oninput={(e) => {
			document.documentElement.style.setProperty("--chat-bg-scrim-color", e.currentTarget.value);
		}}
		onchange={(e) => {
			document.documentElement.style.removeProperty("--chat-bg-scrim-color");
			theme.setScrimColor(e.currentTarget.value);
		}}
	/>
</div>

<style>
	.scrim-colors {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
	}
	.auto {
		height: 24px;
		padding: 0 8px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-full);
		background: var(--surface);
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
		transition:
			border-color var(--duration-fast) var(--ease),
			background var(--duration-fast) var(--ease),
			color var(--duration-fast) var(--ease);
	}
	.auto:hover {
		color: var(--text);
		border-color: var(--accent);
	}
	.auto.on {
		border-color: var(--accent);
		background: var(--accent-soft);
		color: var(--accent-text);
		font-weight: 600;
	}
	.dot,
	.custom {
		width: 22px;
		height: 22px;
		border-radius: 50%;
		border: 2px solid var(--bg-sunken);
		box-shadow: 0 0 0 1px var(--border-strong);
		cursor: pointer;
		padding: 0;
		flex: none;
	}
	.custom {
		background: conic-gradient(from 0deg, #ef4444, #f59e0b, #22c55e, #3b82f6, #a855f7, #ef4444);
		overflow: hidden;
	}
	.custom::-webkit-color-swatch-wrapper {
		padding: 0;
	}
	.custom::-webkit-color-swatch {
		border: none;
		border-radius: 50%;
		opacity: 0;
	}
	.custom.on::-webkit-color-swatch {
		opacity: 1;
	}
	.dot.on,
	.custom.on {
		box-shadow: 0 0 0 2px var(--accent);
	}
</style>
