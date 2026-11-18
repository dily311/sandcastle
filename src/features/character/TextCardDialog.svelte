<script lang="ts">
	// 고른 글 → 이미지 카드. 왼쪽 미리보기, 오른쪽 옵션. 복사(클립보드) 또는 png로 저장
	import { Copy, Download } from "@lucide/svelte";
	import { Button, Dialog, Select, Switch } from "$shared/ui";
	import BackgroundPicker from "$shared/ui/BackgroundPicker.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, saveTo } from "$shared/ipc/dialog";
	import { toasts } from "$shared/stores/toast.svelte";
	import { theme } from "$app/theme.svelte";
	import { dropPaths, isImageAvif } from "$app/drop.svelte";
	import { CARD_DESIGNS, CARD_FONTS, CARD_SIZES, CARD_STYLES, blobBase64, cardBlob, palette, renderCard, type CardDesign, type CardFont, type CardStyle } from "./textCard";

	interface Props {
		card: { text: string; speaker?: string; time?: number } | null;
		onclose: () => void;
	}
	let { card, onclose }: Props = $props();

	// 고른 옵션은 이번 실행 동안 기억 (다음 카드도 같은 모양으로)
	const opts = $state({ design: "footnote" as CardDesign, style: "theme" as CardStyle, font: "app" as CardFont, size: "0", background: null as string | null, speaker: true, time: false });
	// 사용자가 고친 색 (빈 값 = 색 묶음 그대로)
	const colors = $state({ bg: "", text: "", accent: "" });
	const base = $derived(palette(opts.style));
	const COLOR_FIELDS = [
		{ key: "bg", label: "카드" },
		{ key: "text", label: "글자" },
		{ key: "accent", label: "강조" },
	] as const;
	/** input[type=color]는 #rrggbb만 받는다 */
	const hex = (v: string) => (/^#[0-9a-f]{6}$/i.test(v.trim()) ? v.trim() : "#888888");
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
			design: opts.design,
			style: opts.style,
			colors: { bg: colors.bg, text: colors.text, accent: colors.accent },
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

	async function addBackgrounds(paths: string[]) {
		if (!paths.length) return;
		await theme.addBackgrounds(paths, false);
		opts.background = theme.setting.backgrounds?.at(-1) ?? opts.background;
	}
	async function upload() {
		await addBackgrounds(await pickFiles([{ name: "이미지", extensions: ["png", "jpg", "jpeg", "webp", "gif", "avif"] }], true));
	}
	// 카드 창이 열려 있는 동안에는 이 창의 배경으로 (뒤 화면의 끌어다 놓기보다 우선)
	$effect(() => {
		if (!card) return;
		return dropPaths({ priority: 10, label: "카드 배경으로 올리기", hint: "PNG·JPG·WEBP·GIF·AVIF", accept: isImageAvif, onpaths: addBackgrounds });
	});
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
		const data = await blobBase64(await cardBlob(canvas));
		const r = await saveTo("card.png", [{ name: "PNG 이미지", extensions: ["png"] }], async (path) => {
			await call(commands.savePngFile(path, data), { source: "card" });
			return true;
		});
		if (!r) return;
		toasts.success("저장함");
	}
</script>

<Dialog open={!!card} title="이미지 카드" size="lg" {onclose}>
	<div class="layout">
		<div class="preview">{#if url}<img src={url} alt="카드 미리보기" />{/if}</div>
		<div class="opts">
			<label class="field"><span>디자인</span><Select bind:value={opts.design} options={CARD_DESIGNS} aria-label="카드 디자인" /></label>
			<label class="field"><span>색 묶음</span><Select bind:value={opts.style} options={CARD_STYLES} aria-label="카드 색 묶음" /></label>
			<div class="field">
				<span>색 고치기</span>
				<div class="colors">
					{#each COLOR_FIELDS as f (f.key)}
						<label class="swatch" title="{f.label} 색">
							<input type="color" value={hex(colors[f.key] || base[f.key])} oninput={(e) => (colors[f.key] = e.currentTarget.value)} aria-label="{f.label} 색" />
							<span>{f.label}</span>
							{#if colors[f.key]}<button type="button" class="reset" aria-label="{f.label} 색 되돌리기" onclick={() => (colors[f.key] = "")}>×</button>{/if}
						</label>
					{/each}
				</div>
			</div>
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
		width: auto;
		max-width: 100%;
		height: auto;
		max-height: 60vh;
		aspect-ratio: 3 / 4;
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
	.colors {
		display: flex;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
	.swatch {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		color: var(--text);
		font-size: var(--text-xs);
	}
	.swatch input {
		width: 28px;
		height: 28px;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		background: none;
		cursor: pointer;
	}
	.reset {
		padding: 0 4px;
		border: none;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
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
