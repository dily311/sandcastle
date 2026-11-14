<script lang="ts">
	interface Props {
		checked?: boolean;
		label: string;
		/** 라벨 아래 설명 */
		description?: string;
		disabled?: boolean;
		onchange?: (checked: boolean) => void;
		/** 라벨을 화면에 숨기고 낭독기에만 */
		hideLabel?: boolean;
	}
	let { checked = $bindable(false), label, description, disabled = false, onchange, hideLabel = false }: Props =
		$props();
	const id = `sw-${Math.random().toString(36).slice(2, 9)}`;

	function toggle() {
		if (disabled) return;
		checked = !checked;
		onchange?.(checked);
	}
</script>

<div class="switch-row" class:disabled>
	{#if !hideLabel}
		<label class="text" for={id}>
			<span class="label">{label}</span>
			{#if description}<span class="desc">{description}</span>{/if}
		</label>
	{/if}
	<button
		{id}
		type="button"
		role="switch"
		aria-checked={checked}
		aria-label={hideLabel ? label : undefined}
		class="switch"
		class:on={checked}
		{disabled}
		onclick={toggle}
	>
		<span class="thumb"></span>
	</button>
</div>

<style>
	.switch-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
	}
	.text {
		display: flex;
		flex-direction: column;
		gap: 2px;
		cursor: pointer;
	}
	.label {
		font-weight: 500;
	}
	.desc {
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.switch {
		position: relative;
		flex: none;
		width: 38px;
		height: 22px;
		padding: 0;
		border: none;
		border-radius: var(--radius-full);
		background: var(--border-strong);
		cursor: pointer;
		transition: background var(--duration) var(--ease);
	}
	.switch.on {
		background: var(--accent);
	}
	.thumb {
		position: absolute;
		top: 3px;
		left: 3px;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: white;
		box-shadow: var(--shadow-sm);
		transition: transform var(--duration) var(--ease);
	}
	.on .thumb {
		transform: translateX(16px);
	}
	.disabled {
		opacity: 0.5;
	}
	.disabled .switch,
	.disabled .text {
		cursor: not-allowed;
	}
</style>
