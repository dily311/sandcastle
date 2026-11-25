<script lang="ts">
	import { _ } from "svelte-i18n";
	import { theme } from "$app/theme.svelte";

	const PRESETS = $derived<{ value: string; label: string }[]>([
		{ value: "#000000", label: $_("settings.scrim.black") },
		{ value: "#ffffff", label: $_("settings.scrim.white") },
		{ value: "#0f172a", label: $_("settings.scrim.navy") },
		{ value: "#1c102b", label: $_("settings.scrim.deepPurple") },
		{ value: "#221611", label: $_("settings.scrim.warmBrown") },
		{ value: "#0f2318", label: $_("settings.scrim.deepGreen") },
	]);

	const cur = $derived(theme.scrimColor);
	const isCustom = $derived(!!cur && !PRESETS.some((p) => p.value.toLowerCase() === cur.toLowerCase()));
	const pickerValue = $derived(cur ?? (theme.dark ? "#000000" : "#ffffff"));
</script>

<div class="scrim-colors" role="radiogroup" aria-label={$_("settings.scrim.ariaGroup")}>
	<button
		type="button"
		role="radio"
		class="auto"
		class:on={cur === null}
		aria-checked={cur === null}
		title={$_("settings.scrim.autoTitle")}
		onclick={() => theme.setScrimColor(null)}
	>
		{$_("settings.scrim.auto")}
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
		aria-label={$_("settings.scrim.customAria")}
		title={$_("settings.scrim.customTitle")}
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
