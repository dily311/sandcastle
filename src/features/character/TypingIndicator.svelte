<script lang="ts">
	// 응답을 기다릴 때 보이는 타이핑 표시 (5-E-17-1). 대화·폰·설정 미리보기가 같이 쓴다.
	// 모양·글은 설정 > 대화 > 타이핑 표시 (AppSetting.typing). 테마 CSS는 `.sc-typing`으로 고칠 수 있다.
	import type { TypingStyle } from "$ipc/bindings";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { fillNames } from "./format";

	let {
		name = "",
		style,
		text,
		class: cls = "",
	}: {
		/** `{{char}}`에 넣을 이름 */
		name?: string;
		/** 주지 않으면 설정 값 (설정 화면 미리보기는 직접 줌) */
		style?: TypingStyle;
		text?: string;
		class?: string;
	} = $props();

	const shape = $derived(style ?? appSetting.value?.typing?.style ?? "blink");
	const label = $derived(fillNames(text ?? appSetting.value?.typing?.text ?? "", { char: name }).trim());
</script>

{#if shape !== "none" || label}
	<div class="sc-typing {cls}" data-style={shape} role="status" aria-label={label || "응답 기다리는 중"}>
		{#if shape !== "none"}<span class="dots" aria-hidden="true"><span></span><span></span><span></span></span>{/if}
		{#if label}<span class="sc-typing-text">{label}</span>{/if}
	</div>
{/if}

<style>
	.sc-typing {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--typing-color, var(--text-faint));
		font-size: var(--text-sm);
	}
	.dots {
		display: inline-flex;
		gap: 4px;
	}
	.dots span {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: currentColor;
		animation: blink 1.2s infinite;
	}
	[data-style="bounce"] .dots span {
		animation-name: bounce;
		animation-duration: 0.9s;
	}
	.dots span:nth-child(2) {
		animation-delay: 0.2s;
	}
	.dots span:nth-child(3) {
		animation-delay: 0.4s;
	}
	@keyframes blink {
		50% {
			opacity: 0.2;
		}
	}
	@keyframes bounce {
		0%,
		60%,
		100% {
			transform: translateY(0);
		}
		30% {
			transform: translateY(-4px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.dots span {
			animation: none;
		}
	}
</style>
