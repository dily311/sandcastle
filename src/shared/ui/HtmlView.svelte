<script lang="ts">
	// AI 답변의 HTML 코드 블록을 iframe으로 그린다 (JS도 돈다).
	// - 백엔드가 htmlview:// 주소로 서빙하고 그 응답에만 CSP를 붙인다 (sys/html_view.rs)
	// - sandbox="allow-scripts"만: 불투명 origin이라 앱 화면·저장소·Tauri에 닿지 못한다
	// - 높이는 iframe이 postMessage로 알려준다 (최대 MAX_HEIGHT, 넘치면 iframe 안에서 스크롤)
	import { Code, Eye, RotateCw } from "@lucide/svelte";
	import { call, commands } from "$shared/ipc/client";
	import { plugins } from "$shared/stores/plugins.svelte";

	let { code }: { code: string } = $props();

	const MAX_HEIGHT = 900;
	let src = $state<string | null>(null);
	let failed = $state(false);
	let height = $state(120);
	let showCode = $state(false);
	let reloadKey = $state(0);
	let frame: HTMLIFrameElement | undefined = $state();

	/** HTML 렌더링 코어 플러그인이 꺼져 있으면 코드만 보여준다 */
	const enabled = $derived(plugins.on("html"));
	$effect(() => {
		const c = code;
		failed = false;
		if (!enabled) return;
		void call(commands.registerHtmlView(c), { source: "html", silent: true })
			.then((url) => (src = url))
			.catch(() => (failed = true));
	});

	function onMessage(e: MessageEvent) {
		if (!frame || e.source !== frame.contentWindow) return;
		const d = e.data as { type?: string; h?: number };
		if (d?.type === "sc-html-height" && typeof d.h === "number" && d.h > 0) height = Math.min(MAX_HEIGHT, Math.ceil(d.h) + 2);
	}
</script>

<svelte:window onmessage={onMessage} />

<figure class="html">
	<div class="bar">
		<span>HTML{#if !enabled} · 실행 꺼짐 (코어 플러그인){/if}</span>
		<button type="button" title="다시 실행" aria-label="다시 실행" onclick={() => reloadKey++}><RotateCw /></button>
		<button type="button" title={showCode ? "결과 보기" : "코드 보기"} aria-label={showCode ? "결과 보기" : "코드 보기"} aria-pressed={showCode} onclick={() => (showCode = !showCode)}>
			{#if showCode}<Eye />{:else}<Code />{/if}
		</button>
	</div>
	{#if showCode || failed || !enabled}
		<pre><code>{code}</code></pre>
	{:else if src}
		{#key reloadKey}
			<iframe bind:this={frame} {src} title="HTML 미리보기" sandbox="allow-scripts" style:height="{height}px" loading="lazy" referrerpolicy="no-referrer"></iframe>
		{/key}
	{/if}
</figure>

<style>
	.html {
		margin: var(--space-2) 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		overflow: hidden;
		background: var(--surface);
	}
	.bar {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 2px 4px 2px 10px;
		border-bottom: 1px solid var(--border);
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.bar span {
		flex: 1;
	}
	.bar button {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: inherit;
		cursor: pointer;
	}
	.bar button:hover {
		background: var(--surface-hover);
	}
	.bar :global(svg) {
		width: 14px;
		height: 14px;
	}
	iframe {
		display: block;
		width: 100%;
		border: none;
		background: white;
	}
	pre {
		margin: 0;
		max-height: 400px;
		overflow: auto;
		padding: var(--space-2) var(--space-3);
		font-size: var(--text-xs);
		white-space: pre-wrap;
	}
</style>
