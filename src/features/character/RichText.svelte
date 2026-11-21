<script lang="ts">
	// 메시지·캐릭터 개요 공통 본문 (format.ts의 블록을 그린다)
	import { HtmlView } from "$shared/ui";
	import type { Block, Inline } from "./format";

	let { blocks, images = {} }: { blocks: Block[]; images?: Record<string, string> } = $props();
</script>

{#snippet parts(list: Inline[])}{#each list as part, j (j)}{#if part.kind === "action"}<em>{part.text}</em>{:else if part.kind === "speech"}<q>{part.text}</q>{:else if part.kind === "strong"}<strong>{part.text}</strong>{:else if part.kind === "code"}<code>{part.text}</code>{:else}{part.text}{/if}{/each}{/snippet}

{#each blocks as b, i (i)}
	{#if b.kind === "p"}
		<p>{@render parts(b.parts)}</p>
	{:else if b.kind === "h"}
		{#if b.level === 1}<h2 class="rh">{@render parts(b.parts)}</h2>{:else if b.level === 2}<h3 class="rh">{@render parts(b.parts)}</h3>{:else}<h4 class="rh">{@render parts(b.parts)}</h4>{/if}
	{:else if b.kind === "li"}
		<p class="li">{@render parts(b.parts)}</p>
	{:else if b.kind === "quote"}
		<blockquote>{@render parts(b.parts)}</blockquote>
	{:else if b.kind === "hr"}
		<hr />
	{:else if b.kind === "code"}
		<pre class="codeblock" data-lang={b.lang || undefined}><code>{b.text}</code></pre>
	{:else if b.kind === "table"}
		<div class="table-wrap">
			<table>
				<thead><tr>{#each b.head as c, k (k)}<th>{@render parts(c)}</th>{/each}</tr></thead>
				<tbody>{#each b.rows as r, k (k)}<tr>{#each r as c, l (l)}<td>{@render parts(c)}</td>{/each}</tr>{/each}</tbody>
			</table>
		</div>
	{:else if b.kind === "html"}
		<HtmlView code={b.code} />
	{:else if images[b.key]}
		<figure class="cg"><img src={images[b.key]} alt={b.key} loading="lazy" /><figcaption>{b.key}</figcaption></figure>
	{/if}
{/each}

<style>
	p,
	blockquote {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	p + p {
		margin-top: 0.6em;
	}
	em {
		color: var(--text-muted);
	}
	q {
		quotes: none;
		color: var(--text);
		font-weight: 500;
	}
	code {
		padding: 1px 5px;
		border-radius: var(--radius-sm);
		background: var(--surface-2);
		font-family: var(--font-mono, monospace);
		font-size: 0.9em;
	}
	.codeblock {
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
	.codeblock code {
		padding: 0;
		background: none;
		font-size: inherit;
	}
	.table-wrap {
		margin: 0.6em 0;
		overflow-x: auto;
	}
	table {
		border-collapse: collapse;
		font-size: var(--text-sm);
	}
	th,
	td {
		padding: var(--space-1) var(--space-3);
		border: 1px solid var(--border);
		text-align: left;
		vertical-align: top;
	}
	th {
		background: var(--surface-2);
		font-weight: 600;
	}
	.rh {
		margin: 0.9em 0 0.4em;
		line-height: 1.35;
	}
	h2.rh {
		font-size: var(--text-lg);
	}
	h3.rh,
	h4.rh {
		font-size: var(--text-md, 1em);
	}
	.li {
		position: relative;
		padding-left: 1.2em;
	}
	.li + .li {
		margin-top: 0.2em;
	}
	.li::before {
		content: "•";
		position: absolute;
		left: 0.3em;
		color: var(--text-faint);
	}
	blockquote {
		margin: 0.5em 0;
		padding-left: var(--space-3);
		border-left: 3px solid var(--border-strong);
		color: var(--text-muted);
	}
	hr {
		margin: var(--space-3) 0;
		border: none;
		border-top: 1px solid var(--border);
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
