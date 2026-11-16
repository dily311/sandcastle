<script lang="ts">
	// 목록 화면 공통 카드: 위에 그림(커버·아바타·아이콘), 아래 이름 · 설명 · 정보 줄 · 뱃지.
	// 여러 개 고르기 중이면 누를 때 고르기(체크 표시)
	import type { Snippet } from "svelte";

	interface Props {
		title: string;
		desc?: string | null;
		/** 맨 아래 작은 정보 줄 */
		meta?: string;
		/** 그림 주소 (없으면 fallback) */
		image?: string | null;
		/** 그림 칸 비율 (기본 16/10, 그림 목록은 3/4) */
		ratio?: string;
		/** 그림 위치 (커버 위치 조절 값) */
		imagePosition?: string;
		/** 그림이 없을 때 (아바타·아이콘) */
		fallback?: Snippet;
		/** 이름 옆·아래 뱃지 */
		badges?: Snippet;
		/** 카드 아래 추가 동작 (예: 이어서 대화) */
		footer?: Snippet;
		/** 오른쪽 위 버튼 (메뉴 등) */
		corner?: Snippet;
		selecting?: boolean;
		selected?: boolean;
		/** 고를 수 없는 카드 (흐리게) */
		locked?: boolean;
		onclick: () => void;
		oncontextmenu?: (e: MouseEvent) => void;
	}
	let { title, desc, meta, image, ratio = "16 / 10", imagePosition, fallback, badges, footer, corner, selecting = false, selected = false, locked = false, onclick, oncontextmenu }: Props = $props();
</script>

<article class="card" class:selected class:locked={selecting && locked} {oncontextmenu}>
	<button class="main" {onclick} aria-pressed={selecting ? selected : undefined}>
		<span class="media" style:aspect-ratio={ratio}>
			{#if image}
				<img src={image} alt="" loading="lazy" style:object-position={imagePosition} />
			{:else if fallback}
				<span class="fb">{@render fallback()}</span>
			{/if}
			{#if selecting && !locked}<span class="check" aria-hidden="true">{selected ? "✓" : ""}</span>{/if}
		</span>
		<span class="body">
			<b class="title">{title}</b>
			{#if desc}<span class="desc">{desc}</span>{/if}
			{#if badges}<span class="badges">{@render badges()}</span>{/if}
			{#if meta}<span class="meta">{meta}</span>{/if}
		</span>
	</button>
	{#if corner}<div class="corner">{@render corner()}</div>{/if}
	{#if footer}<div class="footer">{@render footer()}</div>{/if}
</article>

<style>
	.card {
		position: relative;
		display: flex;
		flex-direction: column;
		min-width: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		overflow: hidden;
		transition:
			border-color var(--duration-fast) var(--ease),
			box-shadow var(--duration-fast) var(--ease);
	}
	.card:hover {
		border-color: var(--border-strong);
		box-shadow: var(--shadow-sm);
	}
	.card.selected {
		border-color: var(--accent);
		box-shadow: 0 0 0 2px var(--accent);
	}
	.card.locked {
		opacity: 0.45;
	}
	.main {
		display: flex;
		flex-direction: column;
		padding: 0;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.media {
		position: relative;
		display: grid;
		place-items: center;
		width: 100%;
		background: linear-gradient(135deg, var(--accent-soft-hover), var(--bg-sunken));
		color: var(--text-muted);
		overflow: hidden;
	}
	.media img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.fb {
		display: grid;
		place-items: center;
	}
	.fb :global(svg) {
		width: 32px;
		height: 32px;
	}
	.check {
		position: absolute;
		top: 8px;
		left: 8px;
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		border: 2px solid #fff;
		border-radius: 6px;
		background: rgb(0 0 0 / 0.35);
		color: #fff;
		font-size: 13px;
		font-weight: 700;
	}
	.selected .check {
		background: var(--accent);
	}
	.body {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: var(--space-3);
		min-width: 0;
	}
	.title {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.desc {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.badges {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}
	.meta {
		color: var(--text-faint);
		font-size: var(--text-xs);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.corner {
		position: absolute;
		top: 6px;
		right: 6px;
	}
	.footer {
		margin-top: auto;
		border-top: 1px solid var(--border);
	}
</style>
