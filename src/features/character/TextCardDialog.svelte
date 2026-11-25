<script lang="ts">
	// 고른 글 → 이미지 카드. 왼쪽 미리보기, 오른쪽 옵션. 복사(클립보드) 또는 png로 저장
	import { Copy, Download, TextAlignCenter, TextAlignEnd, TextAlignJustify, TextAlignStart } from "@lucide/svelte";
	import { _ } from "svelte-i18n";
	import { getAppLocale } from "$lib/utils/i18n";
	import { Button, Dialog, Select, Switch, Textarea } from "$shared/ui";
	import BackgroundPicker, { type PickerImageTab } from "$shared/ui/BackgroundPicker.svelte";
	import { declarePreviewFonts } from "$shared/ui/userFont";
	import type { GeneratedImage_Serialize as GeneratedImage } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, saveTo } from "$shared/ipc/dialog";
	import { toasts } from "$shared/stores/toast.svelte";
	import { theme } from "$app/theme.svelte";
	import { dropPaths, isImageAvif } from "$app/drop.svelte";
	import { library } from "./library.svelte";
	import { CARD_DESIGNS, CARD_FONTS, CARD_IMG_RATIO, CARD_SIZES, CARD_STYLES, blobBase64, cardBlob, palette, renderCard, maskText, type CardMask, type CardAlign, type CardDesign, type CardFont, type CardImgRatio, type CardStyle } from "./textCard";
	import IconButton from "$/shared/ui/IconButton.svelte";
	import Slider from "$/shared/ui/Slider.svelte";

	interface Props {
		card: { text: string; speaker?: string; time?: number } | null;
		charId?: string;
		convId?: string;
		onclose: () => void;
	}
	let { card, charId, convId, onclose }: Props = $props();

	// 업로드된 폰트도 선택·렌더링 가능하게 미리보기 @font-face 선언
	const uploadedFonts = $derived(theme.setting.fonts ?? []);
	$effect(() => {
		if (!card || !uploadedFonts.length) return;
		return declarePreviewFonts(uploadedFonts);
	});
	const designOptions = $derived(CARD_DESIGNS.map((d) => ({ value: d.value, label: $_(`textCard.design_${d.value}`, { default: d.label }) })));
	const styleOptions = $derived(CARD_STYLES.map((s) => ({ value: s.value, label: $_(`textCard.style_${s.value}`, { default: s.label }) })));
	const fontOptions = $derived([
		...CARD_FONTS.map((f) => ({ value: f.value, label: $_(`textCard.font_${f.value}`, { default: f.label }) })),
		...uploadedFonts.map((f) => ({ value: `user:${f.path}` as CardFont, label: f.name })),
	]);

	// 캐릭터 에셋 + 이 대화방에서 생성된 이미지를 배경 후보 탭으로 제공
	let convGenImages = $state<GeneratedImage[]>([]);
	$effect(() => {
		if (!card) return;
		if (charId && !library.images[charId]) void library.loadImages(charId);
		if (convId) {
			void call(commands.listAllImages(), { source: "card", silent: true })
				.then((all) => (convGenImages = all.filter((i) => i.conversationId === convId)))
				.catch(() => (convGenImages = []));
		} else {
			convGenImages = [];
		}
	});
	const bgExtraTabs = $derived.by<PickerImageTab[]>(() => {
		const tabs: PickerImageTab[] = [];
		const assets = charId ? (library.images[charId] ?? []) : [];
		if (assets.length) {
			tabs.push({ id: "assets", label: $_("textCard.tabCharAssets"), images: assets.map((a) => ({ path: a.path, label: a.key })) });
		}
		if (convGenImages.length) {
			tabs.push({ id: "conv-gen", label: $_("textCard.tabConvGen"), images: convGenImages.map((g) => ({ path: g.path, label: g.name || g.prompt || $_("textCard.genImgFallback") })) });
		}
		return tabs;
	});

	// 고른 옵션은 이번 실행 동안 기억 (다음 카드도 같은 모양으로)
	const opts = $state({ imgRatio: "1:1" as CardImgRatio, design: "none" as CardDesign, style: "theme" as CardStyle, txtShadow: false, font: "app" as CardFont, bold: 500, align: "start" as CardAlign, size: "0", background: null as string | null, speaker: true, time: false });
	// 숨길 말·대체어 (5-E-11): 카드 글에만 적용, 이 기기에 기억 (욕설·금지어 목록은 매번 다시 쓰기 번거로움)
	const MASK_KEY = "sc.card.mask";
	const mask = $state<CardMask>(
		(() => {
			try {
				return { hide: "", replace: "", ...JSON.parse(localStorage.getItem(MASK_KEY) ?? "{}") };
			} catch {
				return { hide: "", replace: "" };
			}
		})(),
	);
	$effect(() => {
		const v = JSON.stringify({ hide: mask.hide, replace: mask.replace });
		try {
			localStorage.setItem(MASK_KEY, v);
		} catch {
			/* 못 쓰면 이번 실행 동안만 */
		}
	});
	// 사용자가 고친 색 (빈 값 = 색 묶음 그대로)
	const colors = $state({ bg: "", text: "", accent: "" });
	const base = $derived(palette(opts.style));
	const COLOR_FIELDS = $derived([
		{ key: "bg", label: $_("textCard.colorCard") },
		{ key: "text", label: $_("textCard.colorText") },
		{ key: "accent", label: $_("textCard.colorAccent") },
	] as const);
	/** input[type=color]는 #rrggbb만 받는다 */
	const hex = (v: string) => (/^#[0-9a-f]{6}$/i.test(v.trim()) ? v.trim() : "#888888");
	let url = $state("");
	let canvas: HTMLCanvasElement | null = null;
	let seq = 0;

	const stamp = (t: number) => new Date(t).toLocaleString(getAppLocale() === "en" ? "en-US" : "ko-KR", { year: "numeric", month: "long", day: "numeric", hour: "2-digit", minute: "2-digit" });

	$effect(() => {
		if (!card) return;
		const input = {
			text: maskText(card.text, { hide: mask.hide, replace: mask.replace }),
			speaker: opts.speaker ? card.speaker : undefined,
			caption: opts.time && card.time ? stamp(card.time) : undefined,
			imgRatio: opts.imgRatio,
			design: opts.design,
			style: opts.style,
			colors: { bg: colors.bg, text: colors.text, accent: colors.accent },
			txtShadow: opts.txtShadow,
			font: opts.font,
			bold: opts.bold,
			align: opts.align,
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
		await addBackgrounds(await pickFiles([{ name: $_("textCard.imageFilterName"), extensions: ["png", "jpg", "jpeg", "webp", "gif", "avif"] }], true));
	}
	// 카드 창이 열려 있는 동안에는 이 창의 배경으로 (뒤 화면의 끌어다 놓기보다 우선)
	$effect(() => {
		if (!card) return;
		return dropPaths({ priority: 10, label: $_("textCard.dropLabel"), hint: "PNG·JPG·WEBP·GIF·AVIF", accept: isImageAvif, onpaths: addBackgrounds });
	});
	async function copy() {
		if (!canvas) return;
		try {
			await navigator.clipboard.write([new ClipboardItem({ "image/png": await cardBlob(canvas) })]);
			toasts.success($_("textCard.copiedToast"));
		} catch (e) {
			toasts.error($_("textCard.copyFailedToast"), String(e));
		}
	}
	async function save() {
		if (!canvas) return;
		const data = await blobBase64(await cardBlob(canvas));
		const r = await saveTo("card.png", [{ name: $_("textCard.pngFilterName"), extensions: ["png"] }], async (path) => {
			await call(commands.savePngFile(path, data), { source: "card" });
			return true;
		});
		if (!r) return;
		toasts.success($_("textCard.savedToast"));
	}
</script>

<Dialog open={!!card} title={$_("textCard.dialogTitle")} size="xl" {onclose}>
	<div class="layout">
		<div class="preview">{#if url}<img src={url} alt={$_("textCard.previewAlt")} />{/if}</div>
		<div class="opts">
			<div class="field">
				<span>{$_("textCard.backgroundLabel")}</span>
				<BackgroundPicker label={$_("textCard.backgroundAria")} value={opts.background} images={theme.setting.backgrounds ?? []} extraTabs={bgExtraTabs} onpick={(v) => (opts.background = v)} onupload={upload} />
			</div>
			<div class="field">
				<span>{$_("textCard.ratioLabel")}</span>
				<div class="group">
					{#each CARD_IMG_RATIO as r}
						<Button variant="ghost" active={opts.imgRatio === r.value} onclick={() => opts.imgRatio = r.value}>
							<span>{r.label}</span>
						</Button>
					{/each}
				</div>
			</div>
			<label class="field"><span>{$_("textCard.designLabel")}</span><Select bind:value={opts.design} options={designOptions} aria-label={$_("textCard.designAria")} /></label>
			<label class="field"><span>{$_("textCard.styleLabel")}</span><Select bind:value={opts.style} options={styleOptions} aria-label={$_("textCard.styleAria")} /></label>
			<div class="field">
				<span>{$_("textCard.customColorsLabel")}</span>
				<div class="colors">
					{#each COLOR_FIELDS as f (f.key)}
						<label class="swatch" title={$_("textCard.colorFieldAria", { values: { label: f.label } })}>
							<input type="color" value={hex(colors[f.key] || base[f.key])} oninput={(e) => (colors[f.key] = e.currentTarget.value)} aria-label={$_("textCard.colorFieldAria", { values: { label: f.label } })} />
							<span>{f.label}</span>
							{#if colors[f.key]}<button type="button" class="reset" aria-label={$_("textCard.colorResetAria", { values: { label: f.label } })} onclick={() => (colors[f.key] = "")}>×</button>{/if}
						</label>
					{/each}
				</div>
			</div>
			<div class="toggle"><span>{$_("textCard.textShadowLabel")}</span><Switch checked={opts.txtShadow} label={$_("textCard.textShadowLabel")} hideLabel onchange={(v) => (opts.txtShadow = v)} /></div>
			<div class="field">
				<span>{$_("textCard.alignLabel")}</span>
				<div class="group">
					<IconButton label={$_("textCard.alignStart")} active={opts.align === "start"} onclick={()=> opts.align = "start"}><TextAlignJustify /></IconButton>
					<IconButton label={$_("textCard.alignLeft")} active={opts.align === "left"} onclick={()=> opts.align = "left"}><TextAlignStart /></IconButton>
					<IconButton label={$_("textCard.alignCenter")} active={opts.align === "center"} onclick={()=> opts.align = "center"}><TextAlignCenter /></IconButton>
					<IconButton label={$_("textCard.alignRight")} active={opts.align === "right"} onclick={()=> opts.align = "right"}><TextAlignEnd /></IconButton>
				</div>
			</div>
			<label class="field"><span>{$_("textCard.fontLabel")}</span><Select bind:value={opts.font} options={fontOptions} aria-label={$_("textCard.fontLabel")} /></label>
			<label class="field">
				<Slider label={$_("textCard.fontSizeLabel")} bind:value={opts.size} min={32} max={64} step={2} fallback={32} unsetLabel="32" />
			</label>
			<label class="field"><span>{$_("textCard.hideWordsLabel")}</span><Textarea bind:value={mask.hide} rows={1} autosize maxRows={4} placeholder={$_("textCard.hideWordsPlaceholder")} aria-label={$_("textCard.hideWordsLabel")} /></label>
			<label class="field"><span>{$_("textCard.replaceWordsLabel")}</span><Textarea bind:value={mask.replace} rows={1} autosize maxRows={4} placeholder={$_("textCard.replaceWordsPlaceholder")} aria-label={$_("textCard.replaceWordsLabel")} /></label>
			{#if card?.speaker}
				<div class="toggle"><span>{$_("textCard.showSpeakerLabel")}</span><Switch checked={opts.speaker} label={$_("textCard.showSpeakerLabel")} hideLabel onchange={(v) => (opts.speaker = v)} /></div>
			{/if}
			{#if card?.time}
				<div class="toggle"><span>{$_("textCard.showTimeLabel")}</span><Switch checked={opts.time} label={$_("textCard.showTimeLabel")} hideLabel onchange={(v) => (opts.time = v)} /></div>
			{/if}
		</div>
	</div>
	{#snippet footer()}
		<Button variant="ghost" onclick={onclose}>{$_("common.close")}</Button>
		<Button onclick={copy}>{#snippet icon()}<Copy />{/snippet}{$_("common.copy")}</Button>
		<Button variant="primary" onclick={save}>{#snippet icon()}<Download />{/snippet}{$_("common.save")}</Button>
	{/snippet}
</Dialog>

<style>
	.layout {
		display: grid;
		grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
		/* grid-template-columns: 1fr; */
		gap: var(--space-4);
		align-items: start;
		overflow: hidden;
		height: 55vh;
	}
	/* 미리보기는 스스로 스크롤하지 않는다 (창 하나만 스크롤) */
	.preview {
		display: grid;
		place-items: center;
		grid-template-rows: minmax(0, 1fr);
		min-height: 160px;
		height: 100%;
		border-radius: var(--radius-md);
		background: var(--bg-sunken);
		overflow: hidden;
	}
	.preview img {
		display: block;
		width: auto;
		max-width: 100%;
		height: auto;
		max-height: 100%;
		object-fit: contain;
	}
	.opts {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		min-width: 0;
		height: 100%;
		overflow-y: auto;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
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
		gap: var(--space-1);
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

	.toggle span {
		font-size: var(--text-sm);
		color: var(--text-muted);
	}

	.group {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}
	@media (max-width: 720px) {
		.layout {
			grid-template-columns: 1fr;
		}

	}
</style>
