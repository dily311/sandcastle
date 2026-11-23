<script lang="ts">
	// 데스크톱에서 띄운 상태창 (5-F-4-2, ADR-029 ⑤): 머리줄을 끌어 옮기고, 오른쪽 아래 모서리로 크기를 바꾼다.
	// 위치·크기·접힘은 이 기기에만 (statusView)
	import { ChevronDown, PanelRightClose } from "@lucide/svelte";
	import { IconButton } from "$shared/ui";
	import type { Block } from "$features/character/format";
	import StatusWindow from "./StatusWindow.svelte";
	import { statusView, type FloatRect } from "./statusView.svelte";

	let { block }: { block: Block | null } = $props();

	const MIN_W = 220;
	const MIN_H = 80;
	const DEFAULT: FloatRect = { x: 0, y: 72, w: 320, h: 260 };

	/** 창 안에 들어오게 (창 크기가 바뀌었거나 저장한 자리가 밖이면) */
	function clamp(r: FloatRect): FloatRect {
		const vw = window.innerWidth;
		const vh = window.innerHeight;
		const w = Math.min(Math.max(MIN_W, r.w), vw - 16);
		const h = Math.min(Math.max(MIN_H, r.h), vh - 16);
		return { w, h, x: Math.min(Math.max(8, r.x || vw - w - 24), vw - w - 8), y: Math.min(Math.max(8, r.y), vh - 40) };
	}
	let rect = $state(clamp(statusView.rect ?? DEFAULT));
	let box: HTMLDivElement | undefined = $state();

	// ---- 옮기기 ----
	let drag: { dx: number; dy: number } | null = null;
	function down(e: PointerEvent) {
		if ((e.target as HTMLElement).closest("button")) return;
		drag = { dx: e.clientX - rect.x, dy: e.clientY - rect.y };
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}
	function move(e: PointerEvent) {
		if (!drag) return;
		rect = clamp({ ...rect, x: e.clientX - drag.dx, y: e.clientY - drag.dy });
	}
	function up() {
		if (!drag) return;
		drag = null;
		statusView.set({ rect });
	}

	// ---- 크기 (CSS resize) ----
	$effect(() => {
		if (!box) return;
		let t: ReturnType<typeof setTimeout> | undefined;
		const ro = new ResizeObserver(() => {
			if (!box || statusView.collapsed) return;
			const w = Math.round(box.offsetWidth);
			const h = Math.round(box.offsetHeight);
			if (w === rect.w && h === rect.h) return;
			rect = { ...rect, w, h };
			clearTimeout(t);
			t = setTimeout(() => statusView.set({ rect }), 300);
		});
		ro.observe(box);
		return () => (ro.disconnect(), clearTimeout(t));
	});
	const onresize = () => (rect = clamp(rect));
</script>

<svelte:window {onresize} />

<div
	bind:this={box}
	class="float sc-status-float"
	class:collapsed={statusView.collapsed}
	role="dialog"
	aria-label="상태창"
	style:left="{rect.x}px"
	style:top="{rect.y}px"
	style:width="{rect.w}px"
	style:height={statusView.collapsed ? "auto" : `${rect.h}px`}
>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="head" onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up}>
		<b>상태</b>
		<IconButton label={statusView.collapsed ? "펼치기" : "접기"} size="sm" aria-expanded={!statusView.collapsed} onclick={() => statusView.set({ collapsed: !statusView.collapsed })}
			><ChevronDown class={statusView.collapsed ? "rot" : ""} /></IconButton
		>
		<IconButton label="패널로 돌리기" size="sm" onclick={() => statusView.set({ floating: false })}><PanelRightClose /></IconButton>
	</div>
	{#if !statusView.collapsed}
		<div class="body"><StatusWindow {block} /></div>
	{/if}
</div>

<style>
	.float {
		position: fixed;
		z-index: var(--z-popover);
		display: flex;
		flex-direction: column;
		min-width: 220px;
		min-height: 40px;
		overflow: hidden;
		resize: both;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: var(--shadow-md);
	}
	.float.collapsed {
		resize: none;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 2px 4px 2px var(--space-3);
		border-bottom: 1px solid var(--border);
		font-size: var(--text-sm);
		cursor: move;
		user-select: none;
		touch-action: none;
	}
	.collapsed .head {
		border-bottom: none;
	}
	.head b {
		flex: 1;
	}
	.head :global(.rot) {
		transform: rotate(-90deg);
	}
	.body {
		flex: 1;
		overflow: auto;
		padding: var(--space-2);
	}
</style>
