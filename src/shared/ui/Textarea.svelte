<script lang="ts">
	import type { HTMLTextareaAttributes } from "svelte/elements";

	interface Props extends Omit<HTMLTextareaAttributes, "value"> {
		value?: string | null;
		invalid?: boolean;
		/** 내용에 맞춰 높이 자동 조절 (maxRows까지) */
		autosize?: boolean;
		maxRows?: number;
		/** 고정폭 글꼴 (프롬프트·정규식 편집) */
		mono?: boolean;
	}
	let {
		value = $bindable(""),
		invalid = false,
		autosize = false,
		maxRows = 12,
		mono = false,
		rows = 3,
		class: className = "",
		...rest
	}: Props = $props();

	let el: HTMLTextAreaElement | undefined = $state();

	$effect(() => {
		if (!autosize || !el) return;
		// 비어 있으면 rows 높이 그대로 (레이아웃 전 좁은 폭에서 잰 값이 굳는 것 방지)
		if (!value) {
			el.style.height = "";
			return;
		}
		el.style.height = "auto";
		const line = parseFloat(getComputedStyle(el).lineHeight) || 20;
		el.style.height = `${Math.min(el.scrollHeight, line * maxRows + 16)}px`;
	});
</script>

<textarea
	bind:this={el}
	bind:value
	{rows}
	class="textarea {className}"
	class:invalid
	class:mono
	aria-invalid={invalid || undefined}
	{...rest}
></textarea>

<style>
	.textarea {
		display: block;
		width: 100%;
		min-height: 1.8rem;
		padding: var(--space-2) var(--space-3);
		background: var(--surface);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		line-height: var(--leading);
		resize: vertical;
		outline: none;
		transition:
			border-color var(--duration-fast) var(--ease),
			box-shadow var(--duration-fast) var(--ease);
	}
	.textarea::placeholder {
		color: var(--text-faint);
	}
	.textarea:focus {
		border-color: var(--accent);
		box-shadow: 0 0 0 3px var(--focus-ring);
	}
	.invalid {
		border-color: var(--danger);
	}
	.mono {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
	}
</style>
