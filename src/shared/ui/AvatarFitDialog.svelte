<script lang="ts">
	// 프로필 그림 확대·위치 맞추기 (5-E-17-3): 끌어서 옮기고, 휠·막대로 확대. 저장은 부르는 쪽이 (onsave)
	import type { AvatarFit } from "$ipc/bindings";
	import Avatar from "./Avatar.svelte";
	import Button from "./Button.svelte";
	import Dialog from "./Dialog.svelte";

	interface Props {
		open: boolean;
		name: string;
		src: string | null;
		color?: string | null;
		fit: AvatarFit | null;
		onclose: () => void;
		/** null = 가운데·확대 없음 */
		onsave: (fit: AvatarFit | null) => void | Promise<void>;
	}
	let { open, name, src, color = null, fit, onclose, onsave }: Props = $props();

	type Fit = { x: number; y: number; zoom: number };
	const SIZE = 200;
	const CENTER: Fit = { x: 50, y: 50, zoom: 1 };
	let cur = $state<Fit>({ ...CENTER });
	$effect(() => {
		if (open) cur = { x: fit?.x ?? 50, y: fit?.y ?? 50, zoom: fit?.zoom ?? 1 };
	});

	const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
	const round = (v: number) => Math.round(v * 10) / 10;

	let drag = $state<{ px: number; py: number; x: number; y: number } | null>(null);
	function down(e: PointerEvent) {
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		drag = { px: e.clientX, py: e.clientY, x: cur.x, y: cur.y };
	}
	function move(e: PointerEvent) {
		if (!drag) return;
		// 그림을 오른쪽으로 끌면 보이는 곳은 왼쪽으로
		const k = 100 / (SIZE * cur.zoom);
		cur = { ...cur, x: round(clamp(drag.x - (e.clientX - drag.px) * k, 0, 100)), y: round(clamp(drag.y - (e.clientY - drag.py) * k, 0, 100)) };
	}
	function up() {
		drag = null;
	}
	function wheel(e: WheelEvent) {
		e.preventDefault();
		cur = { ...cur, zoom: round(clamp(cur.zoom - e.deltaY * 0.002, 1, 4)) };
	}
	function key(e: KeyboardEvent) {
		const d = e.shiftKey ? 10 : 2;
		const step: Record<string, [number, number]> = { ArrowLeft: [d, 0], ArrowRight: [-d, 0], ArrowUp: [0, d], ArrowDown: [0, -d] };
		const s = step[e.key];
		if (!s) return;
		e.preventDefault();
		cur = { ...cur, x: clamp(cur.x + s[0], 0, 100), y: clamp(cur.y + s[1], 0, 100) };
	}
	const isCenter = $derived(cur.x === 50 && cur.y === 50 && cur.zoom === 1);
	let saving = $state(false);
	async function save() {
		saving = true;
		try {
			await onsave(isCenter ? null : { ...cur });
			onclose();
		} finally {
			saving = false;
		}
	}
</script>

<Dialog {open} title="프로필 그림 맞추기" size="sm" {onclose}>
	<div class="fit">
		<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions (방향키로도 옮김) -->
		<div
			class="stage"
			class:dragging={!!drag}
			role="application"
			aria-label="끌거나 방향키로 위치, 휠로 확대"
			tabindex="0"
			onpointerdown={down}
			onpointermove={move}
			onpointerup={up}
			onpointercancel={up}
			onwheel={wheel}
			onkeydown={key}
		>
			<Avatar {name} {src} {color} size={SIZE} fit={cur} />
		</div>
		<div class="zoom">
			<span class="faint" aria-hidden="true">1×</span>
			<input type="range" min="1" max="4" step="0.05" bind:value={cur.zoom} aria-label="확대" />
			<span class="faint" aria-hidden="true">4×</span>
		</div>
		<div class="small">
			<Avatar {name} {src} {color} size={56} fit={cur} />
			<Avatar {name} {src} {color} size={36} fit={cur} />
			<Avatar {name} {src} {color} size={22} fit={cur} />
		</div>
	</div>
	{#snippet footer()}
		<Button variant="ghost" disabled={isCenter} onclick={() => (cur = { ...CENTER })}>가운데로</Button>
		<span class="grow"></span>
		<Button variant="ghost" onclick={onclose}>취소</Button>
		<Button variant="primary" loading={saving} onclick={save}>저장</Button>
	{/snippet}
</Dialog>

<style>
	.fit {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
	}
	.stage {
		border-radius: 50%;
		cursor: grab;
		touch-action: none;
		user-select: none;
	}
	.stage.dragging {
		cursor: grabbing;
	}
	.stage:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 3px;
	}
	.stage :global(img) {
		pointer-events: none;
	}
	.zoom {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
	}
	.zoom input {
		flex: 1;
		accent-color: var(--accent);
	}
	.small {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.faint {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.grow {
		flex: 1;
	}
</style>
