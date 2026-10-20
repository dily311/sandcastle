<script lang="ts">
	import { cn } from "$/lib/utils/cn";
	import type { HTMLAttributes } from "svelte/elements";
	import type { Snippet } from "svelte";

	type VariantType = "horizontal" | "vertical";

	type Props = HTMLAttributes<HTMLDivElement> & {
		variant?: VariantType;
		segmented?: boolean; // 세그먼트 스타일 여부 프롭
		children?: Snippet;
	};

	let { variant = "horizontal", segmented = false, class: className, children, ...restProps }: Props = $props();
</script>

<div role="group" class={cn("btn-group", variant, segmented && "segmented", className)} {...restProps}>
	{@render children?.()}
</div>

<style>
	/* ---------------------------------------
       1. 기본 버튼 그룹 (간격이 떨어져 있는 형태)
       --------------------------------------- */
	.btn-group {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.375rem; /* 버튼 간 살짝 간격 유지 */
		width: fit-content;
		height: fit-content;
	}

	.btn-group.vertical {
		flex-direction: column;
	}

	/* ---------------------------------------
       2. 세그먼트 형태 (서로 딱 달라붙는 그룹)
       --------------------------------------- */
	.btn-group.segmented {
		gap: 0;
		border: 1px solid var(--border-strong, #d1d5db);
		border-radius: var(--radius-sm, 6px);
		overflow: hidden;
	}

	/* 수평 세그먼트일 때 버튼들 간의 구분선 */
	:global(.btn-group.segmented:not(.vertical) .btn + .btn) {
		border-left: 1px solid var(--border-color);
	}

	/* 수직 세그먼트일 때 버튼들 간의 구분선 */
	:global(.btn-group.segmented.vertical .btn + .btn) {
		border-top: 1px solid var(--border-color);
	}

	/* 세그먼트 내부 버튼 초기화 */
	:global(.btn-group.segmented .btn) {
		border-radius: 0;
		border: none;
		background: transparent;
	}

	/* 세그먼트 내부 버튼 호버 시 */
	:global(.btn-group.segmented .btn:hover) {
		background: var(--button-bg-hover, #e5e7eb);
	}

	/* 활성화된 상태 (예: .on 또는 .active) */
	:global(.btn-group.segmented .btn.on),
	:global(.btn-group.segmented .btn.active) {
		
		background: var(--button-bg);
		color: var(--text-color);
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
	}
</style>
