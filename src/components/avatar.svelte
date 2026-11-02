<script lang="ts">
	import { cn } from "$/lib/utils/cn";
	import type { HTMLAttributes } from "svelte/elements";

	type Props = (HTMLAttributes<HTMLDivElement>) & {
		avatar: string | null;
		size?: "sm" | "lg";
	};
	let { avatar, size, class: className, ...restProps }: Props = $props();
</script>

{#if avatar}
	<div class={cn("chat-avatar", className, size)} {...restProps}>
		{#if avatar?.startsWith("http")}
			<img src={avatar} alt="" />
		{:else}
			{avatar}
		{/if}
	</div>
{/if}

<style>
	.chat-avatar {
		width: 2.5rem;
		height: 2.5rem;
		border-radius: 100%;
		flex: 0 0 auto;
		display: inline-flex;
		justify-content: center;
		align-items: center;
		background: var(--card-bg);
		overflow: hidden;
	}
	.chat-avatar.sm {
		width: 14rem;
		height: 14rem;
	}
	.chat-avatar.lg {
		width: 4rem;
		height: 4rem;
	}
	.chat-avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
</style>
