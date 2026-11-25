<script lang="ts">
import { cn } from "$/lib/utils/cn";
import type { Snippet } from "svelte";

	interface Props {
		label: string;
		desc?: string;
		class?: string;
		nowrap?: boolean;
		/** 조작 요소를 아래로 내림 (긴 입력칸) */
		stacked?: boolean;
		children?: Snippet;
	}
	let { label, desc, stacked = false, nowrap=false, class: className, children }: Props = $props();
</script>

<div class={cn("row", className)} class:nowrap class:stacked>
	<div class="txt"><b>{label}</b>{#if desc}<span>{desc}</span>{/if}</div>
	{#if children}<div class="ctl">{@render children()}</div>{/if}
</div>

<style>
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		/* 가로 간격은 좁게 줄 수 있고, 세로 간격(줄바꿈 뒤)은 그대로 */
		gap: var(--space-3) var(--space-6);
		padding: var(--space-4) 0;
		flex-wrap: wrap;
		/* 조작 요소가 행보다 넓어도 잘리지 않게 (예전 overflow: hidden은 좁은 화면에서 오른쪽을 잘라 먹었음) */
		min-width: 0;
		container-type: inline-size;
		container-name: row-container;
	}
	.row.nowrap {
		flex-wrap: nowrap;
	}
	.row + :global(.row) {
		border-top: 1px solid var(--border);
	}
	.row.stacked {
		flex-direction: column;
		align-items: stretch;
		gap: var(--space-3);
	}
	.txt {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.txt b {
		font-weight: 600;
	}
	.txt span {
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.ctl {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-shrink: 0;
		/* 행보다 넓은 조작 요소(색 목록·버튼 둘 이상)는 행 폭 안에서 줄을 바꿈 */
		max-width: 400px;
		flex-wrap: wrap;
	}
	/* 글이 길어도 단어 중간이 아니라 칸 안에서 줄바꿈 */
	.txt {
		overflow-wrap: anywhere;
	}
	.stacked .ctl {
		flex-shrink: 1;
		flex-direction: column;
		align-items: stretch;
	}
	.ctl :global(.select) {
		/* 160px를 바라되 행이 더 좁으면 행 폭까지 */
		min-width: min(160px, 100%);
	}

	@container row-container (max-width: 400px) {
		.ctl {
			max-width: none;
		}
	}
</style>
