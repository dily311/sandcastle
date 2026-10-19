<script lang="ts">
	import { toasts } from "$/lib/stores/toast.svelte";
	import { AlertCircle, CircleCheck, CircleQuestionMarkIcon, Info } from "@lucide/svelte";
</script>

{#if toasts.toastsValue.length > 0}
	<div class="toast-container">
		{#each toasts.toastsValue as toast (toast.id)}
			<div class="toast toast-{toast.type}">
				<div class="toast-icon">
					{#if toast.type === "info"}
						<Info />
					{:else if toast.type === "success"}
						<CircleCheck />
					{:else if toast.type === "error"}
						<AlertCircle />
					{:else}
						<CircleQuestionMarkIcon />
					{/if}
				</div>
				<div class="toast-text">
					{toast.message}
				</div>
			</div>
		{/each}
	</div>
{/if}

<style>
	.toast {
		width: 100%;
		max-width: 240px;
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.4rem 0.6rem;
		font-size: var(--font-size-sm);
		color: var(--text-color);
		border: 1px solid var(--border-color);
		background: var(--background-primary);
		border-radius: var(--radius-sm);
		position: absolute;
		bottom: 0;
		right: 0;
		z-index: 100;
	}

	.toast.info {
		color: var(--color-blue);
	}
	.toast.success {
		color: var(--color-green);
	}
	.toast.error {
		color: var(--color-red);
	}

	.toast-icon {
	}

	.toast-text {
		flex: 1;
	}
</style>
