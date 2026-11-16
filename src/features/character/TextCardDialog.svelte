<script lang="ts">
	// 고른 글 → 이미지 카드. 복사(클립보드) 또는 png로 저장
	import { Copy, Download } from "@lucide/svelte";
	import { Button, Dialog, Segmented } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { pickSavePath } from "$shared/ipc/dialog";
	import { toasts } from "$shared/stores/toast.svelte";
	import { blobBase64, cardBlob, renderCard, type CardStyle } from "./textCard";

	interface Props {
		card: { text: string; speaker?: string; caption?: string } | null;
		onclose: () => void;
	}
	let { card, onclose }: Props = $props();

	let style = $state<CardStyle>("theme");
	let showSpeaker = $state(true);
	let url = $state("");
	let canvas: HTMLCanvasElement | null = null;

	$effect(() => {
		if (!card) return;
		canvas = renderCard({ text: card.text, speaker: showSpeaker ? card.speaker : undefined, caption: card.caption, style });
		const u = canvas.toDataURL("image/png");
		url = u;
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
		const path = await pickSavePath("card.png", [{ name: "PNG 이미지", extensions: ["png"] }]);
		if (!path) return;
		await call(commands.savePngFile(path, await blobBase64(await cardBlob(canvas))), { source: "card" });
		toasts.success("저장함");
	}
</script>

<Dialog open={!!card} title="이미지 카드" description="고른 글을 카드 이미지로 만듦" size="lg" {onclose}>
	<div class="opts">
		<Segmented
			label="카드 색"
			bind:value={style}
			options={[
				{ value: "theme", label: "지금 테마" },
				{ value: "paper", label: "종이" },
				{ value: "night", label: "밤" },
			]}
		/>
		{#if card?.speaker}
			<label class="check"><input type="checkbox" bind:checked={showSpeaker} />이름 넣기</label>
		{/if}
	</div>
	{#if url}<img class="preview" src={url} alt="카드 미리보기" />{/if}
	{#snippet footer()}
		<Button variant="ghost" onclick={onclose}>닫기</Button>
		<Button onclick={copy}>{#snippet icon()}<Copy />{/snippet}복사</Button>
		<Button variant="primary" onclick={save}>{#snippet icon()}<Download />{/snippet}저장</Button>
	{/snippet}
</Dialog>

<style>
	.opts {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		flex-wrap: wrap;
		margin-bottom: var(--space-3);
	}
	.check {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-size: var(--text-sm);
	}
	.preview {
		display: block;
		width: 100%;
		max-height: 60vh;
		object-fit: contain;
		border-radius: var(--radius-md);
		box-shadow: 0 0 0 1px var(--border);
	}
</style>
