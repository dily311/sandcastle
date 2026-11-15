<script lang="ts">
	// 오른쪽 패널 틀: 왼쪽 가장자리를 끌어서 너비 조절 (대화·폰·게임 화면이 같이 씀)
	// 내용은 화면마다 다르고, 탭은 panel.register로 타이틀바에 그린다
	import type { Snippet } from "svelte";
	import { panel } from "./panel.svelte";

	let { label, children }: { label: string; children: Snippet } = $props();

	function startResize(e: PointerEvent) {
		e.preventDefault();
		const startX = e.clientX;
		const startW = panel.width;
		const move = (ev: PointerEvent) => panel.setWidth(startW + (startX - ev.clientX));
		const up = () => {
			panel.setWidth(panel.width, true);
			window.removeEventListener("pointermove", move);
			window.removeEventListener("pointerup", up);
			document.body.style.cursor = "";
		};
		document.body.style.cursor = "col-resize";
		window.addEventListener("pointermove", move);
		window.addEventListener("pointerup", up);
	}
</script>

<div class="side-panel">
	<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions (포커스 가능한 separator는 조절 가능한 위젯) -->
	<div
		class="resize"
		role="separator"
		aria-orientation="vertical"
		aria-label="패널 너비 조절 (←/→)"
		aria-valuenow={panel.width}
		tabindex="0"
		onpointerdown={startResize}
		onkeydown={(e) => {
			if (e.key === "ArrowLeft") panel.setWidth(panel.width + 20, true);
			if (e.key === "ArrowRight") panel.setWidth(panel.width - 20, true);
		}}
		ondblclick={() => panel.setWidth(300, true)}
	></div>
	<aside class="panel" aria-label={label}>
		{@render children()}
	</aside>
</div>

<style>
	.side-panel {
		position: relative;
		display: flex;
		width: var(--panel-width);
		flex: none;
		min-width: 0;
		min-height: 0;
	}
	.panel {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		border-left: 1px solid var(--border);
		background: var(--bg);
		color: var(--text);
	}
	.resize {
		position: absolute;
		top: 0;
		bottom: 0;
		left: -3px;
		z-index: 5;
		width: 6px;
		cursor: col-resize;
	}
	.resize:hover,
	.resize:focus-visible {
		background: color-mix(in oklab, var(--accent) 40%, transparent);
		outline: none;
	}
	/* 패널 안 공통 모양 (각 화면의 패널 내용이 씀) */
	.panel :global(.panel-body) {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		padding: var(--space-3) var(--space-4) var(--space-4);
		overflow: auto;
	}
	.panel :global(.panel-body section) {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.panel :global(.panel-body h4) {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-muted);
		letter-spacing: 0.04em;
		display: flex;
		justify-content: space-between;
		gap: var(--space-2);
	}
	.panel :global(.panel-none) {
		padding: var(--space-4);
		text-align: center;
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
</style>
