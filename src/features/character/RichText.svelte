<script lang="ts">
	// 메시지·캐릭터 개요 공통 본문 (format.ts의 블록을 그린다, 5-E-27 GFM)
	import { HtmlView } from "$shared/ui";
	import { renderUiHtml } from "$shared/html/markdown";
	import type { Block } from "./format";
	import { isStatusPin, pinKey } from "$features/status/pin";
	import { panel } from "$app/panel.svelte";

	/** collapsePins: 상태창 블록을 본문에서 숨기고 고정 자리로 보낸다 (Set이면 선택된 출처만 숨김) */
	let { blocks, images = {}, collapsePins = false }: { blocks: Block[]; images?: Record<string, string>; collapsePins?: boolean | ReadonlySet<string> } = $props();
	const isCollapsedPin = (b: Block) => !!collapsePins && isStatusPin(b) && (collapsePins === true || collapsePins.has(pinKey(b)));
</script>

{#each blocks as b, i (i)}
	{#if isCollapsedPin(b)}
		<!-- <button type="button" class="pin-ref sc-status-pin-ref" title="가장 최근 상태창은 상태 탭(또는 띄운 창·채팅방 위)에" onclick={() => panel.show("status")}>상태창</button> -->
	{:else if b.kind === "md"}
		<!-- 마크다운을 그려 허용 목록으로 거른 HTML (shared/html/markdown.ts) -->
		<div class="md sc-md">{@html b.html}</div>
	{:else if b.kind === "html" && b.scripts === false}
		<!-- 코드 블록 밖 HTML: 안의 글도 마크다운으로, 허용 목록으로 거른 뒤 UI로 바로 그린다 (JS 없음) -->
		<div class="ui-html md sc-md sc-ui-html">{@html renderUiHtml(b.code)}</div>
	{:else if b.kind === "html"}
		<HtmlView code={b.code} />
	{:else if b.kind === "cg" && images[b.key]}
		<figure class="cg sc-cg"><img src={images[b.key]} alt={b.key} loading="lazy" /><figcaption>{b.key}</figcaption></figure>
	{/if}
{/each}

<style>
	.pin-ref {
		display: inline-block;
		margin: 2px 0;
		padding: 1px var(--space-2);
		border: 1px dashed var(--border);
		border-radius: var(--radius-sm);
		background: none;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.md {
		overflow-wrap: anywhere;
		font-family: var(--font-text, var(--font-sans));
	}
	.md + .md {
		margin-top: 0.6em;
	}
	.md :global(p) {
		margin: 0;
	}
	.md :global(p + p),
	.md :global(p + ul),
	.md :global(p + ol),
	.md :global(ul + p),
	.md :global(ol + p) {
		margin-top: 0.6em;
	}
	.md :global(em) {
		color: var(--text-muted);
	}
	.md :global(q) {
		quotes: none;
		color: var(--text);
		font-weight: 500;
	}
	.md :global(del) {
		color: var(--text-faint);
	}
	.md :global(code) {
		padding: 1px 5px;
		border-radius: var(--radius-sm);
		background: var(--surface-2);
		font-family: var(--font-mono, monospace);
		font-size: 0.9em;
	}
	.md :global(pre) {
		margin: 0.6em 0;
		padding: var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--bg-sunken);
		overflow-x: auto;
		white-space: pre;
		font-family: var(--font-mono, monospace);
		font-size: var(--text-sm);
	}
	.md :global(pre code) {
		padding: 0;
		background: none;
		font-size: inherit;
	}
	.md :global(table) {
		display: block;
		max-width: 100%;
		margin: 0.6em 0;
		overflow-x: auto;
		border-collapse: collapse;
		font-size: var(--text-sm);
	}
	.md :global(th),
	.md :global(td) {
		padding: var(--space-1) var(--space-3);
		border: 1px solid var(--border);
		vertical-align: top;
	}
	.md :global(th) {
		background: var(--surface-2);
		font-weight: 600;
	}
	.md :global(h1),
	.md :global(h2),
	.md :global(h3),
	.md :global(h4),
	.md :global(h5),
	.md :global(h6) {
		margin: 0.9em 0 0.4em;
		line-height: 1.35;
		font-size: var(--text-md, 1em);
	}
	.md :global(h1) {
		font-size: var(--text-xl, 1.3em);
	}
	.md :global(h2) {
		font-size: var(--text-lg);
	}
	.md :global(ul),
	.md :global(ol) {
		margin: 0.4em 0;
		padding-left: 1.4em;
	}
	.md :global(li + li) {
		margin-top: 0.2em;
	}
	.md :global(li:has(> .task)) {
		list-style: none;
		margin-left: -1.2em;
	}
	.md :global(.task) {
		color: var(--text-muted);
	}
	.md :global(blockquote) {
		margin: 0.5em 0;
		padding-left: var(--space-3);
		border-left: 3px solid var(--border-strong);
		color: var(--text-muted);
	}
	.md :global(hr) {
		margin: var(--space-3) 0;
		border: none;
		border-top: 1px solid var(--border);
	}
	.md :global(img) {
		max-width: 100%;
	}
	.md :global(.html-link) {
		text-decoration: underline;
		color: var(--accent-text, var(--accent));
	}
	/* 다인봇 말한 사람 (5-F-2) */
	.md :global(p.say) {
		display: flex;
		align-items: baseline;
		flex-wrap: wrap;
		gap: 0 6px;
	}
	.md :global(.who) {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		align-self: center;
	}
	.md :global(.who .av) {
		width: 20px;
		height: 20px;
		border-radius: 50%;
		object-fit: cover;
		display: inline-grid;
		place-items: center;
		background: var(--surface-2);
		font-size: 11px;
		font-weight: 600;
	}
	.md :global(.who b) {
		font-weight: 600;
	}
	/* 코드 블록 밖 HTML (ADR-004 추가) */
	.ui-html {
		position: relative;
		max-width: 100%;
		overflow: hidden;
		contain: layout paint;
	}
	.ui-html :global(details) {
		margin: 0.4em 0;
	}
	.ui-html :global(summary) {
		cursor: pointer;
	}
	.cg {
		margin: var(--space-4) auto;
		width: min(600px, 100%);
		border-radius: var(--radius-lg);
		overflow: hidden;
		border: 1px solid var(--border);
	}
	.cg img {
		display: block;
		width: 100%;
	}
	.cg figcaption {
		padding: 0;
		font-size: var(--text-xs);
		color: var(--text-faint);
		opacity: 0;
		height: 0;
	}
</style>
