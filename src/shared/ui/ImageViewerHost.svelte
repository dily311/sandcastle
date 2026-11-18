<script lang="ts">
	/**
	 * 그림 크게 보기 창 (앱에 하나). imageViewer.open({ src, name })으로 열고, imageZoom.ts가 그림 클릭을 이어 준다.
	 * 그림 아래에 파일 이름, 그림이나 이름에서 오른쪽 클릭(Android는 꾹 누르기)하면 메뉴.
	 * <dialog>로 열어서 다른 대화상자 위에서 눌러도 맨 위에 뜬다.
	 */
	import { Copy, ExternalLink, FileText, X } from "@lucide/svelte";
	import { revealItemInDir } from "@tauri-apps/plugin-opener";
	import { layout } from "$app/layout.svelte";
	import { describeSrc, imageViewer } from "$shared/stores/imageViewer.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";
	import IconButton from "./IconButton.svelte";

	const img = $derived(imageViewer.current);
	const info = $derived(img ? describeSrc(img.src) : null);
	const title = $derived(img?.name || info?.name || "");

	let dialog: HTMLDialogElement | undefined = $state();
	let menu = $state<{ x: number; y: number } | null>(null);

	$effect(() => {
		if (!dialog) return;
		if (img && !dialog.open) dialog.showModal();
		if (!img && dialog.open) dialog.close();
		if (!img) menu = null;
	});

	function oncontextmenu(e: MouseEvent) {
		e.preventDefault();
		menu = { x: e.clientX, y: e.clientY };
	}

	async function copyImage() {
		if (!img) return;
		try {
			const blob = await (await fetch(img.src)).blob();
			// 클립보드는 PNG만 받는 곳이 많아 그림 형식이 다르면 PNG로 바꿔 넣음
			const png = blob.type === "image/png" ? blob : await toPng(blob);
			await navigator.clipboard.write([new ClipboardItem({ "image/png": png })]);
			toasts.success("그림을 복사함");
		} catch (e) {
			toasts.error("그림을 복사할 수 없음", String(e));
		}
	}
	async function toPng(blob: Blob): Promise<Blob> {
		const bmp = await createImageBitmap(blob);
		const canvas = document.createElement("canvas");
		canvas.width = bmp.width;
		canvas.height = bmp.height;
		canvas.getContext("2d")?.drawImage(bmp, 0, 0);
		return new Promise((resolve, reject) => canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("변환 실패"))), "image/png"));
	}
	async function copyName() {
		try {
			await navigator.clipboard.writeText(title);
			toasts.success("파일 이름을 복사함");
		} catch (e) {
			toasts.error("복사할 수 없음", String(e));
		}
	}
	async function reveal() {
		if (!info?.path) return;
		await revealItemInDir(info.path).catch((e) => toasts.error("파일 위치를 열 수 없음", String(e)));
	}

	const items = $derived<MenuItem[]>([
		{ label: "그림 복사", icon: Copy, onselect: copyImage },
		{ label: "파일 이름 복사", icon: FileText, disabled: !title, onselect: copyName },
		// 파일 위치 열기는 데스크톱의 내 파일만 (Android에는 탐색기가 없고, 인터넷 그림은 파일이 아님)
		...(layout.desktop && info?.path ? [{ label: "파일 위치 열기", icon: ExternalLink, separator: true, onselect: reveal }] : []),
	]);
</script>

<dialog bind:this={dialog} class="image-viewer" aria-label="그림 크게 보기" onclose={() => imageViewer.close()} onclick={(e) => e.target === dialog && imageViewer.close()}>
	{#if img}
		<figure>
			<div class="top"><IconButton label="닫기" size="sm" onclick={() => imageViewer.close()}><X /></IconButton></div>
			<img src={img.src} alt={title} {oncontextmenu} />
			{#if title}<figcaption {oncontextmenu}>{title}</figcaption>{/if}
		</figure>
	{/if}
	<ContextMenu at={menu} {items} label="그림 메뉴" onclose={() => (menu = null)} />
</dialog>

<style>
	.image-viewer {
		width: 100vw;
		max-width: 100vw;
		height: 100vh;
		height: 100dvh;
		max-height: none;
		margin: 0;
		padding: var(--space-4);
		border: none;
		background: transparent;
		color: var(--text);
		overflow: hidden;
	}
	.image-viewer[open] {
		display: grid;
		place-items: center;
	}
	.image-viewer::backdrop {
		background: rgb(0 0 0 / 0.72);
	}
	figure {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-2);
		max-width: 100%;
		max-height: 100%;
		margin: 0;
	}
	.top {
		align-self: flex-end;
	}
	img {
		display: block;
		min-height: 0;
		max-width: 100%;
		/* 위 닫기 줄(약 40px)과 아래 이름 줄(약 32px)을 뺀 높이 */
		max-height: calc(100dvh - var(--space-4) * 2 - 80px - var(--safe-top, 0px) - var(--safe-bottom, 0px));
		object-fit: contain;
		border-radius: var(--radius-md);
		/* 눌러서 닫히는 배경과 달리 그림은 길게 눌러 메뉴를 열 수 있게 */
		-webkit-touch-callout: none;
	}
	figcaption {
		max-width: min(100%, 640px);
		padding: var(--space-1) var(--space-3);
		border-radius: var(--radius-sm);
		background: var(--surface);
		font-size: var(--text-sm);
		color: var(--text-muted);
		text-align: center;
		overflow-wrap: anywhere;
	}
</style>
