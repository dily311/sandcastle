<script lang="ts">
	/**
	 * 그림 크게 보기 창 (앱에 하나). imageViewer.open({ src, name })으로 열고, imageZoom.ts가 그림 클릭을 이어 준다.
	 * 그림 아래에 파일 이름, 그림이나 이름에서 오른쪽 클릭(Android는 꾹 누르기)하면 메뉴.
	 * <dialog>로 열어서 다른 대화상자 위에서 눌러도 맨 위에 뜬다.
	 */
	import { Copy, Ellipsis, EllipsisVertical, ExternalLink, FileText, Info, Pencil, Trash2, X } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import { revealItemInDir } from "@tauri-apps/plugin-opener";
	import { layout } from "$app/layout.svelte";
	import { describeSrc, imageViewer } from "$shared/stores/imageViewer.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";
	import Dialog from "./Dialog.svelte";
	import IconButton from "./IconButton.svelte";

	const img = $derived(imageViewer.current);
	const info = $derived(img ? describeSrc(img.src) : null);
	const title = $derived(img?.name || info?.name || "");

	let dialog: HTMLDialogElement | undefined = $state();
	let menu = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	let imgEl: HTMLImageElement | undefined = $state();
	/** 정보 보기 줄 (열 때 계산) */
	let details = $state<{ label: string; value: string }[] | null>(null);

	$effect(() => {
		if (!dialog) return;
		if (img && !dialog.open) dialog.showModal();
		if (!img && dialog.open) dialog.close();
		if (!img) {
			menu = null;
			details = null;
		}
	});
	// 다른 그림으로 바뀌면 정보는 닫음
	$effect(() => {
		void img?.src;
		details = null;
	});
	function openMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.right, y: r.bottom + 4, align: "end" };
	}
	const KB = 1024;
	const sizeText = (n: number) => (n >= KB * KB ? `${(n / KB / KB).toFixed(1)}MB` : `${Math.max(1, Math.round(n / KB))}KB`);
	async function showInfo() {
		if (!img) return;
		const rows: { label: string; value: string }[] = [];
		if (title) rows.push({ label: $_("sharedControls.viewerName"), value: title });
		if (info?.path) rows.push({ label: $_("sharedControls.viewerPath"), value: info.path });
		if (imgEl?.naturalWidth) rows.push({ label: $_("sharedControls.viewerSize"), value: `${imgEl.naturalWidth} × ${imgEl.naturalHeight}px` });
		const ext = /\.([a-z0-9]+)$/i.exec(title)?.[1];
		if (ext) rows.push({ label: $_("sharedControls.viewerFormat"), value: ext.toUpperCase() });
		details = [...rows, ...(img.info ?? [])];
		// 용량은 읽어 봐야 알 수 있어 뒤늦게 덧붙임
		try {
			const blob = await (await fetch(img.src)).blob();
			if (details && imageViewer.current === img) details = [...details, { label: $_("sharedControls.viewerFileSize"), value: sizeText(blob.size) }];
		} catch {
			/* 못 읽으면 생략 */
		}
	}

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
			toasts.success($_("sharedControls.viewerCopiedImg"));
		} catch (e) {
			toasts.error($_("sharedControls.viewerCopyImgErr"), String(e));
		}
	}
	async function toPng(blob: Blob): Promise<Blob> {
		const bmp = await createImageBitmap(blob);
		const canvas = document.createElement("canvas");
		canvas.width = bmp.width;
		canvas.height = bmp.height;
		canvas.getContext("2d")?.drawImage(bmp, 0, 0);
		return new Promise((resolve, reject) => canvas.toBlob((b) => (b ? resolve(b) : reject(new Error($_("sharedControls.viewerConvertErr")))), "image/png"));
	}
	async function copyName() {
		try {
			await navigator.clipboard.writeText(title);
			toasts.success($_("sharedControls.viewerCopiedName"));
		} catch (e) {
			toasts.error($_("sharedControls.viewerCopyErr"), String(e));
		}
	}
	async function reveal() {
		if (!info?.path) return;
		await revealItemInDir(info.path).catch((e) => toasts.error($_("sharedControls.viewerRevealErr"), String(e)));
	}

	const extra = $derived<MenuItem[]>(typeof img?.extraItems === "function" ? img.extraItems() : (img?.extraItems ?? []));
	const items = $derived<MenuItem[]>([
		{ label: $_("viewer.copyImage"), icon: Copy, onselect: copyImage },
		{ label: $_("viewer.copyName"), icon: FileText, disabled: !title, onselect: copyName },
		{ label: $_("viewer.showInfo"), icon: Info, onselect: showInfo },
		...(extra.length ? [{ ...extra[0], separator: true }, ...extra.slice(1)] : []),
		...(img?.rename ? [{ label: $_("common.rename"), icon: Pencil, separator: true, onselect: () => void img.rename?.() }] : []),
		// 파일 위치 열기는 데스크톱의 내 파일만 (Android에는 탐색기가 없고, 인터넷 그림은 파일이 아님)
		...(layout.desktop && info?.path ? [{ label: $_("viewer.reveal"), icon: ExternalLink, separator: true, onselect: reveal }] : []),
		...(img?.remove ? [{ label: $_("common.delete"), icon: Trash2, danger: true, separator: true, onselect: () => void img.remove?.() }] : []),
	]);
</script>

<dialog bind:this={dialog} class="image-viewer" aria-label={$_("sharedControls.viewerAria")} onclose={() => imageViewer.close()} onclick={(e) => e.target === dialog && imageViewer.close()}>
	{#if img}
		<figure>
			<div class="top">
				<div class="cap" role="group" aria-label={$_("sharedControls.viewerFilenameAria")}>
					<span class="name">{title || $_("sharedControls.viewerImgFallback")}</span>
				</div>
				<div class="actionBtn">
					<IconButton label={$_("sharedControls.viewerMenuAria")} size="sm" variant="static-white" onclick={openMenu}><EllipsisVertical /></IconButton>
					<IconButton label={$_("common.close")} size="sm" variant="static-white" onclick={() => imageViewer.close()}><X /></IconButton>	
				</div>
			</div>
			<img bind:this={imgEl} src={img.src} alt={title} {oncontextmenu} />
		</figure>
	{/if}
	<ContextMenu at={menu} {items} label={$_("sharedControls.viewerMenuAria")} onclose={() => (menu = null)} />
</dialog>

<!-- 정보는 그림 위에 따로 뜨는 창 (그림이 크면 아래에 붙여서는 화면 밖으로 나감) -->
<Dialog open={!!details} title={$_("sharedControls.viewerInfoTitle")} size="md" onclose={() => (details = null)}>
	<dl class="details">
		{#each details ?? [] as d, i (`${d.label}-${i}`)}
			<dt>{d.label}</dt>
			<dd>
				<span>{d.value}</span>
				{#if d.value.length > 24}
					<button
						type="button"
						class="copy-val"
						aria-label={$_("sharedControls.viewerCopyFieldAria", { values: { label: d.label } })}
						title={$_("sharedControls.viewerCopyFieldAria", { values: { label: d.label } })}
						onclick={() => navigator.clipboard.writeText(d.value).then(() => toasts.success($_("sharedControls.viewerCopiedField", { values: { label: d.label } })), () => toasts.error($_("sharedControls.viewerCopyFailed")))}
					><Copy /></button>
				{/if}
			</dd>
		{/each}
	</dl>
</Dialog>

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
		width: 100%;
		display: flex;
		justify-content: space-between;
	}
	/* 파일 이름 */
	.cap {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		max-width: min(100%, 640px);
		font-size: var(--text-sm);
		color: var(--static-white);
	}
	.name {
		min-width: 0;
		overflow-wrap: anywhere;
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
	.details {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: 8px var(--space-3);
		margin: 0;
		font-size: var(--text-sm);
	}
	.details dt {
		color: var(--text-faint);
		white-space: nowrap;
	}
	.details dd {
		margin: 0;
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
		color: var(--text);
		overflow-wrap: anywhere;
		white-space: pre-wrap;
		user-select: text;
	}
	.details dd span {
		flex: 1;
		min-width: 0;
	}
	.copy-val {
		flex: none;
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		padding: 0;
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
	}
	.copy-val:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.copy-val :global(svg) {
		width: 14px;
		height: 14px;
	}
</style>
