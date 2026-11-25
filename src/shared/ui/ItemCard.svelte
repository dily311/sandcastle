<script lang="ts">
	// 목록 화면 공통 카드: 위에 그림(커버·아바타·아이콘), 아래 이름(+ 옆 메뉴 버튼) · 설명 · 정보 줄 · 뱃지.
	// 카드 전체가 눌림. 여러 개 고르기 중이면 누를 때 고르기(체크 표시)
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
		/** 뱃지 */
		badges?: Snippet;
		/** 카드 아래 추가 동작 (예: 이어서 대화) */
		footer?: Snippet;
		/** 이름 옆 버튼 (… 메뉴) */
		menu?: Snippet;
		selecting?: boolean;
		selected?: boolean;
		/** 고를 수 없는 카드 (흐리게) */
		locked?: boolean;
		onclick: () => void;
		oncontextmenu?: (e: MouseEvent) => void;
	}
	let { title, desc, meta, image, ratio = "16 / 10", imagePosition, fallback, badges, footer, menu, selecting = false, selected = false, locked = false, onclick, oncontextmenu }: Props = $props();

	function onkeydown(e: KeyboardEvent) {
		if (e.target !== e.currentTarget) return;
		if (e.key === "Enter" || e.key === " ") {
			e.preventDefault();
			onclick();
		}
	}
	// 안쪽 버튼(메뉴·이어서)을 누르면 카드 클릭으로 번지지 않게
	const stop = (e: Event) => e.stopPropagation();
</script>

<!-- 카드 전체가 버튼 역할. 안쪽에 다른 버튼(메뉴·이어서)이 있어 button 대신 role 사용 -->
<div class="card" class:selected class:locked={selecting && locked} role="button" tabindex="0" aria-pressed={selecting ? selected : undefined} aria-label={title} {onclick} {onkeydown} {oncontextmenu}>
	<div class="media" style:aspect-ratio={ratio}>
		{#if image}
			<img src={image} alt="" loading="lazy" style:object-position={imagePosition} />
		{:else if fallback}
			<span class="fb">{@render fallback()}</span>
		{/if}
		{#if selecting && !locked}<span class="check" aria-hidden="true">{selected ? "✓" : ""}</span>{/if}
	</div>
	<div class="body">
		<div class="titlerow">
			<b class="title" title={title}>{title}</b>
			<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (안쪽 버튼이 키보드를 받음) -->
			{#if menu && !selecting}<span class="menu" onclick={stop}>{@render menu()}</span>{/if}
		</div>
		{#if desc}<span class="desc">{desc}</span>{/if}
		{#if badges}<span class="badges">{@render badges()}</span>{/if}
		{#if meta}<span class="meta">{meta}</span>{/if}
	</div>
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (안쪽 버튼이 키보드를 받음) -->
	{#if footer}<div class="footer" onclick={stop}>{@render footer()}</div>{/if}
</div>

<style>
	.card {
		position: relative;
		display: flex;
		flex-direction: column;
		min-width: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		color: var(--text);
		overflow: hidden;
		cursor: pointer;
		/* 꾹 누르기(터치)는 메뉴를 연다 — 글자가 선택되거나 시스템 메뉴가 뜨지 않게 */
		-webkit-user-select: none;
		user-select: none;
		-webkit-touch-callout: none;
		transition:
			transform var(--duration-fast) var(--ease),
			border-color var(--duration-fast) var(--ease),
			box-shadow var(--duration-fast) var(--ease);
	}
	.card:hover {
		border-color: var(--border-strong);
		box-shadow: var(--shadow-md);
		transform: translateY(-2px);
	}
	.card:focus-visible {
		outline: 2px solid var(--focus-ring);
		outline-offset: 2px;
	}
	.card.selected {
		border-color: var(--accent);
		box-shadow: 0 0 0 2px var(--accent);
	}
	.card.locked {
		opacity: 0.45;
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
		transition: transform 240ms var(--ease);
	}
	.card:hover .media img {
		transform: scale(1.03);
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
	.titlerow {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		min-width: 0;
	}
	.title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.menu {
		flex-shrink: 0;
		margin: -4px -6px -4px 0;
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
	.footer {
		margin-top: auto;
		cursor: default;
	}
	/* 카드 아래 버튼은 카드 폭 그대로 (예: 이어서 대화) */
	.footer > :global(button) {
		display: block;
		width: 100%;
		text-align: left;
		font: inherit;
	}
</style>
