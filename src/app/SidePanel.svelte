<script lang="ts">
	// 오른쪽 패널 틀: 왼쪽 가장자리를 끌어서 너비 조절 (대화·폰·게임 화면이 같이 씀)
	// 내용은 화면마다 다르고, 탭은 panel.register로 타이틀바에 그린다
	import type { Snippet } from "svelte";
	import { X } from "@lucide/svelte";
	import { fade, fly } from "svelte/transition";
	import IconButton from "$shared/ui/IconButton.svelte";
	import { layout } from "./layout.svelte";
	import { panel } from "./panel.svelte";
	import PluginPanelView from "$features/community/PluginPanelView.svelte";
	import { isPluginTab } from "$features/community/panels";

	let { label, children }: { label: string; children: Snippet } = $props();

	// 탭이 많아 넘치면 휠·끌기로 옆으로 넘김 (좁은 화면에서도 창을 키우지 않고 다른 탭으로 가게)
	function wheelTabs(e: WheelEvent) {
		const el = e.currentTarget as HTMLElement;
		if (el.scrollWidth <= el.clientWidth || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
		e.preventDefault();
		el.scrollLeft += e.deltaY;
	}

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

{#if layout.narrow}
	<!-- 휴대폰: 화면 전체를 덮는 패널. 탭은 위에, 닫기는 오른쪽 위 -->
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (닫기 버튼이 따로 있음) -->
	<div class="sheet-scrim" transition:fade={{ duration: 150 }} onclick={() => panel.toggle()}></div>
	<div class="sheet" transition:fly={{ x: 400, duration: 220 }}>
		<header class="sheet-head">
			<div class="stabs" role="tablist" aria-label={label} onwheel={wheelTabs}>
				{#each panel.allTabs as t (t.id)}
					<button role="tab" class:on={panel.tab === t.id} aria-selected={panel.tab === t.id} onclick={() => (panel.tab = t.id)} {...panel.tabDrag(t.id)}>
						<t.icon /><span>{t.label}</span>
					</button>
				{/each}
			</div>
			<IconButton label="패널 닫기" onclick={() => panel.toggle()}><X /></IconButton>
		</header>
		<aside class="panel sc-side-panel" aria-label={label}>
			{#if isPluginTab(panel.tab)}<PluginPanelView tab={panel.tab} />{/if}
			<div class="screen-panel" class:hidden={isPluginTab(panel.tab)}>{@render children()}</div>
		</aside>
	</div>
{:else}
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
	<aside class="panel sc-side-panel" aria-label={label}>
		{#if isPluginTab(panel.tab)}<PluginPanelView tab={panel.tab} />{/if}
			<div class="screen-panel" class:hidden={isPluginTab(panel.tab)}>{@render children()}</div>
	</aside>
</div>
{/if}

<style>
	.screen-panel {
		display: contents;
	}
	.screen-panel.hidden {
		display: none;
	}
	.sheet-scrim {
		position: fixed;
		inset: 0;
		z-index: var(--z-sheet);
		background: rgb(0 0 0 / 0.4);
	}
	.sheet {
		position: fixed;
		left: 0;
		right: 0;
		bottom: 0;
		top: 0;
		z-index: calc(var(--z-sheet) + 1);
		display: flex;
		flex-direction: column;
		background: var(--bg);
		overflow: hidden;
		padding: var(--safe-top) env(safe-area-inset-right) var(--safe-bottom) env(safe-area-inset-left);
	}
	.sheet-head {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		min-height: 52px;
		padding: 0 var(--space-1) 0 var(--space-2);
		border-bottom: 1px solid var(--border);
	}
	.stabs {
		display: flex;
		gap: 4px;
		flex: 1;
		min-width: 0;
		overflow-x: auto;
		scrollbar-width: none;
	}
	.stabs::-webkit-scrollbar {
		display: none;
	}
	.stabs button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		flex: none;
		height: 34px;
		padding: 0 12px;
		border: none;
		border-radius: var(--radius-full);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-sm);
		cursor: pointer;
	}
	.stabs button :global(svg) {
		width: 15px;
		height: 15px;
	}
	.stabs button.on {
		background: var(--surface-active);
		color: var(--text);
		font-weight: 600;
	}
	.sheet .panel {
		border-left: none;
		overflow: auto;
	}
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
		z-index: var(--z-float);
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
