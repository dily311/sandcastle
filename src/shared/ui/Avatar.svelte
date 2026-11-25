<script lang="ts">
	import type { AvatarFit, AvatarFrameShape } from "$ipc/bindings";
	interface Props {
		name: string;
		/** 표시할 수 있는 URL (convertFileSrc 결과 등) */
		src?: string | null;
		size?: number;
		/** 지정하지 않으면 이름에서 색을 정한다 */
		color?: string | null;
		/** 온라인/새 메시지 표시 */
		status?: "none" | "online" | "unread";
		/** 그림의 어느 곳을 보일지 (5-E-17-3): x·y = 그림 안 위치(%), zoom = 확대 */
		fit?: AvatarFit | null;
		/** 프로필 프레임 모양 (원/정사각형/사각형/커스텀) */
		shape?: Exclude<AvatarFrameShape, "none">;
		/** 커스텀 프레임 이미지 URL (shape === "custom"일 때 위에 겹쳐 그림) */
		frameSrc?: string | null;
	}
	let { name, src = null, size = 36, color = null, status = "none", fit = null, shape = "circle", frameSrc = null }: Props = $props();

	let failed = $state(false);
	$effect(() => {
		void src;
		failed = false;
	});
	const pos = $derived(fit ? `${fit.x ?? 50}% ${fit.y ?? 50}%` : null);
	const zoom = $derived(fit?.zoom ?? 1);
	const initial = $derived((name.trim()[0] ?? "?").toUpperCase());
	// 이름 → 색상 (같은 이름은 항상 같은 색)
	const hue = $derived([...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 17));
	const bg = $derived(color ?? `oklch(0.72 0.09 ${hue})`);
	const height = $derived(shape === "rect" ? Math.round(size * 1.33) : size);
</script>

<span
	class="avatar sc-avatar shape-{shape}"
	class:has-frame={shape === "custom" && !!frameSrc}
	style:width="{size}px"
	style:height="{height}px"
	style:font-size="{Math.round(size * 0.42)}px"
	style:--avatar-bg={bg}
>
	<span class="avatar-inner">
		{#if src && !failed}
			<img
				class="avatar-img"
				{src}
				alt=""
				style:object-position={pos}
				style:transform={zoom !== 1 ? `scale(${zoom})` : null}
				style:transform-origin={pos}
				onerror={() => (failed = true)}
			/>
		{:else}
			<span class="initial" aria-hidden="true">{initial}</span>
		{/if}
	</span>
	{#if shape === "custom" && frameSrc}
		<img class="avatar-frame" src={frameSrc} alt="" aria-hidden="true" />
	{/if}
	<span class="sr-only">{name}</span>
	{#if status !== "none"}<span class="dot {status}"></span>{/if}
</span>

<style>
	.avatar {
		position: relative;
		display: inline-flex;
		justify-content: center;
		place-items: center;
		flex: none;
		border-radius: 50%;
		background: var(--avatar-bg);
		color: white;
		font-weight: 700;
		overflow: hidden;
	}
	.avatar.shape-square,
	.avatar.shape-rect {
		border-radius: var(--radius-xs);
	}
	.avatar.shape-custom {
		border-radius: var(--radius-xs);
		overflow: visible;
		background: transparent;
	}
	.avatar-inner {
		display: inline-flex;
		justify-content: center;
		align-items: center;
		width: 100%;
		height: 100%;
		border-radius: inherit;
		background: var(--avatar-bg);
		overflow: hidden;
	}
	.avatar.shape-custom.has-frame .avatar-inner {
		width: 82%;
		height: 82%;
	}
	.avatar-img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		object-position: center;
	}
	.avatar-frame {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: contain;
		pointer-events: none;
		z-index: 1;
	}
	.initial {
		text-shadow: 0 1px 2px rgb(0 0 0 / 0.15);
	}
	.dot {
		position: absolute;
		right: -1px;
		bottom: -1px;
		width: 30%;
		height: 30%;
		min-width: 8px;
		min-height: 8px;
		border-radius: 50%;
		border: 2px solid var(--surface);
	}
	.online {
		background: var(--success);
	}
	.unread {
		background: var(--accent);
	}
</style>
