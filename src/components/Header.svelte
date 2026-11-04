<script lang="ts">
	import { cn } from "$/lib/utils/cn";
	import type { HTMLAttributes } from "svelte/elements";

	type Props = HTMLAttributes<HTMLDivElement>;
	let { class: className, children }: Props = $props();
</script>

<div class={cn("header", className)}>
	<!-- 타이틀 바 -->

	{@render children?.()}
</div>

<style>
	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		height: var(--header-height);
		flex: 0 0 var(--header-height);
		border-bottom: 1px solid var(--border-color);
		padding: 0 7rem 0 0.95rem;
		background: var(--header-bg);
		white-space: nowrap;
		position: sticky;
		top: 0;
	}
	.header.isRightSidebarOpen {
		padding: 0 0.95rem;
    }
</style>
