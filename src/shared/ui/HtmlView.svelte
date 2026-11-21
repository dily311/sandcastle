<script lang="ts">
	// AI 답변의 HTML 코드 블록을 iframe으로 그린다 (JS도 돈다).
	// - 백엔드가 htmlview:// 주소로 서빙하고 그 응답에만 CSP를 붙인다 (sys/html_view.rs)
	// - sandbox="allow-scripts"만: 불투명 origin이라 앱 화면·저장소·Tauri에 닿지 못한다
	// - 높이는 iframe이 postMessage로 알려준다 (최대 MAX_HEIGHT, 넘치면 iframe 안에서 스크롤)
	import { ChevronDown, Code, Eye, RotateCw } from "@lucide/svelte";
	import { call, commands } from "$shared/ipc/client";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { theme } from "$app/theme.svelte";
	import { currentThemeCss } from "$shared/html/theme";
	import { dispatchAction, parseAction } from "$shared/html/bridge";
	import { toasts } from "$shared/stores/toast.svelte";

	/** scripts: false면 글쓴이의 JS를 막고 그림만 (코드 블록 밖 HTML, ADR-004 추가) */
	let { code, scripts = true }: { code: string; scripts?: boolean } = $props();

	const MAX_HEIGHT = 900;
	let src = $state<string | null>(null);
	let failed = $state(false);
	let height = $state(120);
	let showCode = $state(false);
	/** 접으면 머리줄만 남긴다 (iframe도 내린다) */
	let folded = $state(false);
	let reloadKey = $state(0);
	let frame: HTMLIFrameElement | undefined = $state();

	/** HTML 렌더링 코어 플러그인이 꺼져 있으면 코드만 보여준다 */
	const enabled = $derived(plugins.on("html"));
	$effect(() => {
		const c = code;
		failed = false;
		if (!enabled) return;
		// 지금 테마 색을 문서에 넣는다. 테마(모드·강조색 등)가 바뀌면 다시 등록해 새 색으로
		void theme.dark;
		void JSON.stringify(theme.setting);
		const css = currentThemeCss(theme.dark);
		void call(commands.registerHtmlView(c, css, scripts), { source: "html", silent: true })
			.then((url) => (src = url))
			.catch(() => (failed = true));
	});

	function onMessage(e: MessageEvent) {
		if (!frame || e.source !== frame.contentWindow) return;
		const d = e.data as { type?: string; h?: number };
		if (d?.type === "sc-html-height" && typeof d.h === "number" && d.h > 0) height = Math.min(MAX_HEIGHT, Math.ceil(d.h) + 2);
		const req = parseAction(e.data);
		if (!req) return;
		// 보내기는 요금이 나간다: 사용자가 이 블록 안을 막 눌렀을 때만 (iframe에 포커스). 아니면 입력칸에 넣기만
		const clicked = document.activeElement === frame;
		const action = req.action === "send" && !clicked ? "fill" : req.action;
		if (!dispatchAction({ action, text: req.text })) return void toasts.info("대화 화면에서만 쓸 수 있음");
		if (req.action === "send" && action === "fill") toasts.info("입력칸에 넣음 · 직접 보내기");
	}
</script>

<svelte:window onmessage={onMessage} />

<figure class="html">
	<div class="bar" class:folded>
		<button type="button" class="fold" title={folded ? "펼치기" : "접기"} aria-label={folded ? "HTML 펼치기" : "HTML 접기"} aria-expanded={!folded} onclick={(e) => { e.stopPropagation(); folded = !folded; }}>
			<ChevronDown class={folded ? "rot" : ""} />
			<span>HTML{#if !enabled} · 실행 꺼짐 (코어 플러그인){:else if !scripts} · 스크립트 없이{/if}</span>
		</button>
		<button type="button" title="다시 실행" aria-label="다시 실행" onclick={() => reloadKey++}><RotateCw /></button>
		<button type="button" title={showCode ? "결과 보기" : "코드 보기"} aria-label={showCode ? "결과 보기" : "코드 보기"} aria-pressed={showCode} onclick={() => (showCode = !showCode)}>
			{#if showCode}<Eye />{:else}<Code />{/if}
		</button>
	</div>
	{#if folded}
		<!-- 접힘 -->
	{:else if showCode || failed || !enabled}
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
		padding: 2px 4px;
		border-bottom: 1px solid var(--border);
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.bar :first-child {
		flex: 1;
	}
	.bar.folded {
		border-bottom: none;
	}
	.bar .fold {
		display: flex;
		gap: var(--space-1);
	}
	.bar .fold :global(svg) {
		transition: transform var(--duration-fast) var(--ease);
	}
	.bar .fold :global(.rot) {
		transform: rotate(-90deg);
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
	.bar :global(svg) {
		width: 14px;
		height: 14px;
	}
	iframe {
		display: block;
		width: 100%;
		border: none;
		background: var(--surface);
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
