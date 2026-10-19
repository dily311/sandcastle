<script>
	export let text = "입력 중";
</script>

<div class="typing-container">
	{#if text}
		<span class="text">{text}</span>
	{/if}
	<div class="dots">
		<span class="dot"></span>
		<span class="dot"></span>
		<span class="dot"></span>
	</div>
</div>

<style>
	.typing-container {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
	}

	.dots {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.dot {
		width: 6px;
		height: 6px;
		background-color: var(--indicator-color);
		border-radius: 50%;
		animation: bounce 1.4s infinite ease-in-out both;
	}

	.dot:nth-child(1) {
		animation-delay: -0.32s;
	}

	.dot:nth-child(2) {
		animation-delay: -0.16s;
	}

	@keyframes bounce {
		0%,
		80%,
		100% {
			transform: scale(0);
			opacity: 0.4;
		}
		40% {
			transform: scale(1);
			opacity: 1;
		}
	}

	.text {
		font-size: 0.8125rem;
		color: var(--text-muted);
	}
</style>
