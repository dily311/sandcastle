<script lang="ts">
	let { size = 16, label = "불러오는 중" }: { size?: number; label?: string } = $props();
</script>

<span class="spinner" style:width="{size}px" style:height="{size}px" role="status" aria-label={label}></span>

<style>
	.spinner {
		display: inline-block;
		border: 2px solid currentColor;
		border-right-color: transparent;
		border-radius: 50%;
		opacity: 0.7;
		animation: spin 0.7s linear infinite;
		flex: none;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.spinner {
			animation-duration: 2s;
		}
	}
</style>
