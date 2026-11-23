<script lang="ts">
	interface Props {
		name: string;
		/** 표시할 수 있는 URL (convertFileSrc 결과 등) */
		src?: string | null;
		size?: number;
		/** 지정하지 않으면 이름에서 색을 정한다 */
		color?: string | null;
		/** 온라인/새 메시지 표시 */
		status?: "none" | "online" | "unread";
	}
	let { name, src = null, size = 36, color = null, status = "none" }: Props = $props();

	let failed = $state(false);
	const initial = $derived((name.trim()[0] ?? "?").toUpperCase());
	// 이름 → 색상 (같은 이름은 항상 같은 색)
	const hue = $derived([...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 17));
	const bg = $derived(color ?? `oklch(0.72 0.09 ${hue})`);
</script>

<span
	class="avatar sc-avatar"
	style:width="{size}px"
	style:height="{size}px"
	style:font-size="{Math.round(size * 0.42)}px"
	style:--avatar-bg={bg}
>
	{#if src && !failed}
		<img {src} alt="" onerror={() => (failed = true)} />
	{:else}
		<span class="initial" aria-hidden="true">{initial}</span>
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
	img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		object-position: center;
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
