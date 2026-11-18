<script lang="ts">
	// 앱 전체에서 파일 끌어다 놓기를 받는 덮개. 지금 화면이 등록한 대상(drops.target)에 넘긴다.
	// 화면 안의 끌어서 옮기기(대화 목록 정렬 등)는 파일이 아니므로 건드리지 않는다.
	import { FileDown, Ban } from "@lucide/svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { drops } from "./drop.svelte";

	let depth = 0;
	const hasFiles = (e: DragEvent) => !!e.dataTransfer && Array.from(e.dataTransfer.types).includes("Files");

	function track(e: DragEvent) {
		const el = document.elementFromPoint(e.clientX, e.clientY);
		if (el !== drops.over) drops.over = el;
	}
	function enter(e: DragEvent) {
		if (!hasFiles(e)) return;
		depth++;
		drops.dragging = true;
		track(e);
	}
	function over(e: DragEvent) {
		if (!hasFiles(e)) return;
		// 기본 동작(웹뷰가 파일을 열어 버림)을 늘 막음
		e.preventDefault();
		track(e);
		if (e.dataTransfer) e.dataTransfer.dropEffect = drops.target ? "copy" : "none";
	}
	function leave(e: DragEvent) {
		if (!hasFiles(e)) return;
		depth = Math.max(0, depth - 1);
		if (depth === 0) {
			drops.dragging = false;
			drops.over = null;
		}
	}
	async function drop(e: DragEvent) {
		if (!hasFiles(e)) return;
		e.preventDefault();
		depth = 0;
		drops.dragging = false;
		track(e);
		const t = drops.target;
		drops.over = null;
		const files = Array.from(e.dataTransfer?.files ?? []);
		if (!t || !files.length) return;
		const ok = files.filter(t.accept);
		const skipped = files.length - ok.length;
		if (skipped) toasts.info(ok.length ? `${skipped}개는 여기서 쓸 수 없어 뺐음` : `여기서는 이 파일을 쓸 수 없음${t.hint ? ` (${t.hint})` : ""}`);
		if (ok.length) await t.ondrop(ok);
	}
</script>

<svelte:window ondragenter={enter} ondragover={over} ondragleave={leave} ondrop={drop} />

{#if drops.dragging}
	{@const zone = drops.target?.zone?.()?.getBoundingClientRect()}
	<div
		class="layer"
		class:none={!drops.target}
		class:zoned={!!zone}
		style:left={zone ? `${zone.left}px` : undefined}
		style:top={zone ? `${zone.top}px` : undefined}
		style:width={zone ? `${zone.width}px` : undefined}
		style:height={zone ? `${zone.height}px` : undefined}
		aria-hidden="true"
	>
		<div class="box">
			{#if drops.target}
				<FileDown />
				<b>{drops.target.label}</b>
				{#if drops.target.hint}<span>{drops.target.hint}</span>{/if}
			{:else}
				<Ban />
				<b>이 화면에서는 파일을 놓을 수 없음</b>
			{/if}
		</div>
	</div>
{/if}

<style>
	.layer {
		position: fixed;
		inset: 0;
		z-index: var(--z-drop);
		display: grid;
		place-items: center;
		padding: var(--space-6);
		background: color-mix(in oklab, var(--bg) 70%, transparent);
		backdrop-filter: blur(2px);
		pointer-events: none;
		animation: fade var(--duration-fast) var(--ease);
	}
	/* 영역이 있는 대상: 그 영역만 덮음 */
	.layer.zoned {
		inset: auto;
		padding: var(--space-3);
	}
	.box {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-2);
		width: min(440px, 100%);
		padding: var(--space-8) var(--space-6);
		border: 2px dashed var(--accent);
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-lg);
		text-align: center;
		color: var(--accent-text);
	}
	.none .box {
		border-color: var(--border-strong);
		color: var(--text-muted);
	}
	.box :global(svg) {
		width: 32px;
		height: 32px;
	}
	.box b {
		font-size: var(--text-lg);
		color: var(--text);
	}
	.box span {
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	@keyframes fade {
		from {
			opacity: 0;
		}
	}
</style>
