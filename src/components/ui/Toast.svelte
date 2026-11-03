<script lang="ts">
	import { toasts } from "$/lib/stores/toast.svelte";
	import Icon from "./Icon.svelte";
	import { writeText } from "@tauri-apps/plugin-clipboard-manager";
	import { cn } from "$/lib/utils/cn";

	const toastIconType = (type: string) => {
		switch (type) {
			case "info":
				return "info";
			case "success":
				return "CircleCheck";
			case "error":
				return "AlertCircle";
			default:
				return "CircleQuestionMarkIcon";
		}
	};

	async function handleClick(mes: string) {
		try {
			await writeText(mes);
			alert("클립보드에 복사되었습니다!");
		} catch (err) {
			console.error("복사 실패:", err);
		}
	}
</script>

{#if toasts.toastsValue.length > 0}
	<div class="toast-container">
		{#each toasts.toastsValue as toast (toast.id)}
			<div role="none" class={cn("toast", toast.type)} onclick={() => handleClick(toast.message)}>
				<div class="toast-title-wrap">
					<div class="toast-icon">
						<Icon name={toastIconType(toast.type)} size={14} />
					</div>
					<div class="toast-title">
						{toast.type}
					</div>
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
		flex-direction: column;
		gap: 0.4rem;
		padding: 0.4rem 0.6rem;
		font-size: var(--font-size-sm);
		color: var(--text-color);
		border: 1px solid var(--border-color);
		background: var(--background-primary);
		border-radius: var(--radius-sm);
		position: absolute;
		bottom: 10px;
		right: 10px;
		z-index: 100;
		overflow: hidden;
	}

	.toast-title-wrap {
		display: flex;
		align-items: center;
		gap: 4px;
	}
	.toast-title {
		font-weight: bold;
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
	.toast-text {
		flex: 1;
	}
</style>
