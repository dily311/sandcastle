<script lang="ts">
	// 배경 고르기: 처음엔 한 줄(올리기 · 없음 · 그라데이션 · 올린 이미지), "더보기"를 누르면 격자로 전부.
	// 값: null(없음) / "preset:<id>" / 이미지 경로
	import { Camera, Ban, ChevronDown, X } from "@lucide/svelte";
	import { fileSrc } from "$shared/ipc/file";
	import { GRADIENTS, gradientCss } from "./backgrounds";

	interface Props {
		value: string | null;
		/** 올려 둔 이미지 경로들 */
		images: string[];
		onpick: (value: string | null) => void;
		/** 이미지 올리기 (없으면 올리기 칸을 숨김) */
		onupload?: () => void;
		/** 올린 이미지 지우기 (없으면 지우기 버튼 숨김) */
		onremove?: (path: string) => void;
		/** "없음" 칸 보이기 */
		allowNone?: boolean;
		label?: string;
	}
	let { value, images, onpick, onupload, onremove, allowNone = true, label = "배경" }: Props = $props();

	let expanded = $state(false);
	let row: HTMLDivElement | undefined = $state();
	let overflowing = $state(false);
	const count = $derived((onupload ? 1 : 0) + (allowNone ? 1 : 0) + GRADIENTS.length + images.length);

	// 한 줄에 다 들어가면 "더보기"를 숨긴다
	$effect(() => {
		void count;
		const el = row;
		if (!el || expanded) return;
		// 크기 감시 콜백 안에서 바로 화면을 바꾸면 "ResizeObserver loop" 알림이 나므로 다음 프레임에
		let frame = 0;
		const check = () => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => (overflowing = el.scrollWidth > el.clientWidth + 1));
		};
		check();
		const ro = new ResizeObserver(check);
		ro.observe(el);
		return () => {
			cancelAnimationFrame(frame);
			ro.disconnect();
		};
	});
</script>

<div class="picker" class:expanded role="radiogroup" aria-label={label}>
	<div class="tiles" bind:this={row}>
		{#if onupload}
			<button type="button" class="tile upload" title="이미지 올리기" aria-label="이미지 올리기" onclick={onupload}><Camera /></button>
		{/if}
		{#if allowNone}
			<button type="button" class="tile none" class:on={!value} role="radio" aria-checked={!value} title="없음" aria-label="배경 없음" onclick={() => onpick(null)}><Ban /></button>
		{/if}
		{#each images as p (p)}
			<span class="wrap">
				<button type="button" class="tile" class:on={value === p} role="radio" aria-checked={value === p} title="올린 이미지" aria-label="올린 이미지" onclick={() => onpick(p)}>
					<img src={fileSrc(p)} alt="" loading="lazy" />
				</button>
				{#if onremove && expanded}<button type="button" class="del" aria-label="이 배경 지우기" title="지우기" onclick={() => onremove(p)}><X /></button>{/if}
			</span>
		{/each}
		{#each GRADIENTS as g (g.id)}
			{@const v = `preset:${g.id}`}
			<button type="button" class="tile" class:on={value === v} role="radio" aria-checked={value === v} title={g.name} aria-label={g.name} style:background={gradientCss(g)} onclick={() => onpick(v)}></button>
		{/each}
	</div>
	{#if overflowing || expanded}
		<button type="button" class="more" aria-expanded={expanded} onclick={() => (expanded = !expanded)}>
			{expanded ? "접기" : "더보기"}<ChevronDown class={expanded ? "up" : ""} />
		</button>
	{/if}
</div>

<style>
	.picker {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;
		max-width: 100%;
	}
	.picker.expanded {
		flex-direction: column;
		align-items: stretch;
	}
	.tiles {
		display: flex;
		gap: 8px;
		min-width: 0;
		flex: 1;
		overflow: hidden;
		padding: 3px;
	}
	.expanded .tiles {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(48px, 1fr));
		overflow: visible;
		max-height: 260px;
		overflow-y: auto;
	}
	.wrap {
		position: relative;
		display: block;
	}
	.tile {
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		width: 48px;
		height: 48px;
		padding: 0;
		border: none;
		border-radius: var(--radius-md);
		background: var(--surface-active);
		color: var(--text-muted);
		cursor: pointer;
		overflow: hidden;
		box-shadow: 0 0 0 1px var(--border);
	}
	/* 칸은 항상 정사각형 */
	.expanded .tile {
		width: 100%;
		height: auto;
		aspect-ratio: 1;
	}
	.tile:hover {
		filter: brightness(1.05);
	}
	.tile.on {
		box-shadow:
			0 0 0 2px var(--surface),
			0 0 0 4px var(--accent);
	}
	.tile img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.tile :global(svg) {
		width: 18px;
		height: 18px;
	}
	.upload {
		background: var(--text-faint);
		color: var(--surface);
	}
	.del {
		position: absolute;
		top: -6px;
		right: -6px;
		display: grid;
		place-items: center;
		width: 18px;
		height: 18px;
		padding: 0;
		border: none;
		border-radius: 50%;
		background: var(--text);
		color: var(--surface);
		cursor: pointer;
	}
	.del :global(svg) {
		width: 11px;
		height: 11px;
	}
	.more {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		flex-shrink: 0;
		align-self: center;
		padding: 4px 8px;
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.expanded .more {
		align-self: flex-end;
	}
	.more:hover {
		background: var(--surface-hover);
	}
	.more :global(svg) {
		width: 14px;
		height: 14px;
		transition: transform var(--duration-fast) var(--ease);
	}
	.more :global(.up) {
		transform: rotate(180deg);
	}
</style>
