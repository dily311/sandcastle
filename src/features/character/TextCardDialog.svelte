<script lang="ts">
	// 고른 글 → 이미지 카드. 왼쪽 미리보기, 오른쪽 옵션. 복사(클립보드) 또는 png로 저장
	import { Copy, Download } from "@lucide/svelte";
	import { Button, Dialog, Select, Switch } from "$shared/ui";
	import BackgroundPicker from "$shared/ui/BackgroundPicker.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, pickSavePath } from "$shared/ipc/dialog";
	import { toasts } from "$shared/stores/toast.svelte";
	import { theme } from "$app/theme.svelte";
	import { CARD_FONTS, CARD_SIZES, CARD_STYLES, blobBase64, cardBlob, renderCard, type CardFont, type CardStyle } from "./textCard";

	interface Props {
		card: { text: string; speaker?: string; time?: number } | null;
		onclose: () => void;
	}
	let { card, onclose }: Props = $props();

	// 고른 옵션은 이번 실행 동안 기억 (다음 카드도 같은 모양으로)
	const opts = $state({ style: "theme" as CardStyle, font: "app" as CardFont, size: "0", background: null as string | null, speaker: true, time: false });
	let url = $state("");
	let canvas: HTMLCanvasElement | null = null;
	let seq = 0;

	const stamp = (t: number) => new Date(t).toLocaleString("ko-KR", { year: "numeric", month: "long", day: "numeric", hour: "2-digit", minute: "2-digit" });

	$effect(() => {
		if (!card) return;
		const input = {
			text: card.text,
			speaker: opts.speaker ? card.speaker : undefined,
			caption: opts.time && card.time ? stamp(card.time) : undefined,
			style: opts.style,
			font: opts.font,
			size: Number(opts.size),
			background: opts.background,
		};
		const my = ++seq;
		void renderCard(input).then((cv) => {
			if (my !== seq) return;
			canvas = cv;
			url = cv.toDataURL("image/png");
		});
	});

	async function upload() {
		const paths = await pickFiles([{ name: "이미지", extensions: ["png", "jpg", "jpeg", "webp", "gif", "avif"] }], true);
		if (!paths.length) return;
		await theme.addBackgrounds(paths, false);
		opts.background = theme.setting.backgrounds?.at(-1) ?? opts.background;
	}
	async function copy() {
		if (!canvas) return;
		try {
			await navigator.clipboard.write([new ClipboardItem({ "image/png": await cardBlob(canvas) })]);
			toasts.success("이미지를 복사함");
		} catch (e) {
			toasts.error("복사 못 함", String(e));
		}
	}
	async function save() {
		if (!canvas) return;
		const path = await pickSavePath("card.png", [{ name: "PNG 이미지", extensions: ["png"] }]);
		if (!path) return;
		await call(commands.savePngFile(path, await blobBase64(await cardBlob(canvas))), { source: "card" });
		toasts.success("저장함");
	}
</script>

<Dialog open={!!card} title="이미지 카드" size="lg" {onclose}>
	<div class="layout">
		<div class="preview">{#if url}<img src={url} alt="카드 미리보기" />{/if}</div>
		<div class="opts">
			<label class="field"><span>색</span><Select bind:value={opts.style} options={CARD_STYLES} aria-label="카드 색" /></label>
			<label class="field"><span>글꼴</span><Select bind:value={opts.font} options={CARD_FONTS} aria-label="글꼴" /></label>
			<label class="field"><span>글자 크기</span><Select bind:value={opts.size} options={CARD_SIZES} aria-label="글자 크기" /></label>
			<div class="field">
				<span>배경</span>
				<BackgroundPicker label="카드 배경" value={opts.background} images={theme.setting.backgrounds ?? []} onpick={(v) => (opts.background = v)} onupload={upload} />
			</div>
			{#if card?.speaker}
				<div class="toggle"><span>이름 넣기</span><Switch checked={opts.speaker} label="이름 넣기" hideLabel onchange={(v) => (opts.speaker = v)} /></div>
			{/if}
			{#if card?.time}
				<div class="toggle"><span>답변 시간 표시</span><Switch checked={opts.time} label="답변 시간 표시" hideLabel onchange={(v) => (opts.time = v)} /></div>
			{/if}
		</div>
	</div>
	{#snippet footer()}
		<Button variant="ghost" onclick={onclose}>닫기</Button>
		<Button onclick={copy}>{#snippet icon()}<Copy />{/snippet}복사</Button>
		<Button variant="primary" onclick={save}>{#snippet icon()}<Download />{/snippet}저장</Button>
	{/snippet}
</Dialog>

<style>
	.layout {
		display: grid;
		grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
		gap: var(--space-4);
		align-items: start;
	}
	/* 미리보기는 스스로 스크롤하지 않는다 (창 하나만 스크롤) */
	.preview {
		display: grid;
		place-items: center;
		min-height: 160px;
		border-radius: var(--radius-md);
		background: var(--bg-sunken);
		overflow: hidden;
	}
	.preview img {
		display: block;
		width: 100%;
		height: auto;
		max-height: 52vh;
		object-fit: contain;
	}
	.opts {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		min-width: 0;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
		font-size: var(--text-sm);
		color: var(--text-muted);
		min-width: 0;
	}
	.toggle {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: var(--text-sm);
	}
	@media (max-width: 720px) {
		.layout {
			grid-template-columns: 1fr;
		}
	}
</style>
