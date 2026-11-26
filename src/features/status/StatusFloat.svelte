<script lang="ts">
	// 데스크톱에서 띄운 상태창 (5-F-4-2, ADR-029 ⑤): 머리줄을 끌어 옮기고, 오른쪽 아래 모서리로 크기를 바꾼다.
	// separateWindows 옵션이 켜져 있고 상태창이 여러 개면 각각 독립된 팝업창으로 띄운다.
	import { ChevronDown, PanelRightClose } from "@lucide/svelte";
	import { _ } from "$lib/utils/i18n";
	import { IconButton } from "$shared/ui";
	import type { Block } from "$features/character/format";
	import StatusWindow from "./StatusWindow.svelte";
	import { statusView, type FloatRect } from "./statusView.svelte";

	let { blocks }: { blocks: Block[] } = $props();

	const MIN_W = 220;
	const MIN_H = 80;

	interface WinSpec {
		key: string;
		title: string;
		blocks: Block[];
		index: number;
	}
	const windows = $derived<WinSpec[]>(
		statusView.separateWindows && blocks.length > 1
			? blocks.map((b, i) => ({ key: `win:${i}`, title: $_("statusUi.winNumbered", { values: { n: i + 1 } }), blocks: [b], index: i }))
			: [{ key: "single", title: $_("statusUi.winSingle"), blocks, index: 0 }],
	);

	function defaultRect(index: number): FloatRect {
		const w = 320;
		const h = 260;
		const vw = typeof window !== "undefined" ? window.innerWidth : 1280;
		return {
			x: Math.max(8, vw - w - 24 - index * 28),
			y: 72 + index * 38,
			w,
			h,
		};
	}

	/** 창 안에 들어오게 (창 크기가 바뀌었거나 저장한 자리가 밖이면) */
	function clamp(r: FloatRect, index = 0): FloatRect {
		const vw = window.innerWidth;
		const vh = window.innerHeight;
		const w = Math.min(Math.max(MIN_W, r.w), vw - 16);
		const h = Math.min(Math.max(MIN_H, r.h), vh - 16);
		const def = defaultRect(index);
		return {
			w,
			h,
			x: Math.min(Math.max(8, r.x || def.x), vw - w - 8),
			y: Math.min(Math.max(8, r.y), vh - 40),
		};
	}

	let rects = $state<Record<string, FloatRect>>({});
	function rectOf(win: WinSpec): FloatRect {
		if (rects[win.key]) return rects[win.key];
		const saved = win.key === "single" ? statusView.rect : statusView.getRect(win.key);
		return clamp(saved ?? defaultRect(win.index), win.index);
	}
	function saveRect(win: WinSpec, r: FloatRect) {
		rects[win.key] = r;
		if (win.key === "single") statusView.set({ rect: r });
		else statusView.setRect(win.key, r);
	}
	function isCollapsed(win: WinSpec): boolean {
		return win.key === "single" ? statusView.collapsed : statusView.isCollapsed(win.key);
	}
	function toggleCollapsed(win: WinSpec) {
		const next = !isCollapsed(win);
		if (win.key === "single") statusView.set({ collapsed: next });
		else statusView.setCollapsed(win.key, next);
	}

	// ---- 옮기기 ----
	let drag: { key: string; index: number; dx: number; dy: number } | null = null;
	let topKey = $state<string>("single");
	function down(e: PointerEvent, win: WinSpec) {
		topKey = win.key;
		if ((e.target as HTMLElement).closest("button")) return;
		const r = rectOf(win);
		drag = { key: win.key, index: win.index, dx: e.clientX - r.x, dy: e.clientY - r.y };
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}
	function move(e: PointerEvent, win: WinSpec) {
		if (!drag || drag.key !== win.key) return;
		const r = rectOf(win);
		rects[win.key] = clamp({ ...r, x: e.clientX - drag.dx, y: e.clientY - drag.dy }, win.index);
	}
	function up(win: WinSpec) {
		if (!drag || drag.key !== win.key) return;
		drag = null;
		saveRect(win, rectOf(win));
	}

	// ---- 크기 (CSS resize) ----
	function trackResize(node: HTMLDivElement, win: WinSpec) {
		let cur = win;
		let t: ReturnType<typeof setTimeout> | undefined;
		const ro = new ResizeObserver(() => {
			if (isCollapsed(cur)) return;
			const r = rectOf(cur);
			const w = Math.round(node.offsetWidth);
			const h = Math.round(node.offsetHeight);
			if (w === r.w && h === r.h) return;
			const next = { ...r, w, h };
			rects[cur.key] = next;
			clearTimeout(t);
			t = setTimeout(() => saveRect(cur, next), 300);
		});
		ro.observe(node);
		return {
			update(nextWin: WinSpec) {
				cur = nextWin;
			},
			destroy() {
				ro.disconnect();
				clearTimeout(t);
			},
		};
	}

	function onresize() {
		const next: Record<string, FloatRect> = {};
		for (const w of windows) next[w.key] = clamp(rectOf(w), w.index);
		rects = next;
	}
</script>

<svelte:window {onresize} />

{#each windows as win (win.key)}
	{@const r = rectOf(win)}
	{@const col = isCollapsed(win)}
	<div
		use:trackResize={win}
		class="float sc-status-float"
		class:collapsed={col}
		class:front={topKey === win.key}
		role="dialog"
		tabindex="-1"
		aria-label={win.title}
		style:left="{r.x}px"
		style:top="{r.y}px"
		style:width="{r.w}px"
		style:height={col ? "auto" : `${r.h}px`}
		onpointerdown={() => (topKey = win.key)}
	>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="head" onpointerdown={(e) => down(e, win)} onpointermove={(e) => move(e, win)} onpointerup={() => up(win)} onpointercancel={() => up(win)}>
			<b>{win.title}</b>
			<IconButton label={col ? $_("statusUi.expand") : $_("statusUi.collapse")} size="sm" aria-expanded={!col} onclick={() => toggleCollapsed(win)}><ChevronDown class={col ? "rot" : ""} /></IconButton>
			<IconButton label={$_("statusUi.dockToPanel")} size="sm" onclick={() => statusView.set({ floating: false })}><PanelRightClose /></IconButton>
		</div>
		{#if !col}
			<div class="body"><StatusWindow blocks={win.blocks} /></div>
		{/if}
	</div>
{/each}

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
		box-shadow: var(--shadow-lg);
	}
	.float.front {
		z-index: calc(var(--z-popover) + 1);
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
		background: var(--surface-2);
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
