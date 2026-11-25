<script lang="ts">
	import { _ } from "svelte-i18n";
	// 폰 카메라 (5-E-19-2): 실제 카메라처럼 — 위에 프리셋 이름 줄, 가운데 그림, 아래 장면 글과 촬영 버튼.
	// 프리셋의 세부 값(모델·크기·프롬프트)은 설정 > 이미지 생성에서. 찍을 때만 그림 API를 부른다(유료).
	import { onMount } from "svelte";
	import { ChevronLeft, Settings } from "@lucide/svelte";
	import type { GeneratedImage_Serialize as GeneratedImage, ImageGenSetting_Serialize as ImageGenSetting } from "$ipc/bindings";
	import { Spinner } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { fileSrc } from "$shared/ipc/file";
	import { nav } from "$app/nav.svelte";
	import { library } from "$features/character/library.svelte";
	import { pluginSetting } from "$shared/stores/aiTools.svelte";
	import { presetFromMeta } from "$features/images/presetFromImage";

	let { album, onback, onalbum }: { album: string; onback: () => void; onalbum: () => void } = $props();
	const src = { source: "imagegen" };

	/** #프리셋 태그를 단 그림 = 카메라 모드 */
	let presetImages = $state<GeneratedImage[]>([]);
	/** 고른 모드 ("" = 설정의 프리셋) */
	let mode = $state("");
	let prompt = $state("");
	/** 대필: AI가 장면 글을 그림 프롬프트로 바꿈 (채팅 호출 한 번 더) */
	let ghost = $state(false);
	let shooting = $state(false);
	let shots = $state<GeneratedImage[]>([]);
	let current = $state<GeneratedImage | null>(null);

	onMount(() => {
		if (!pluginSetting.value) void pluginSetting.load();
		if (!library.loaded) void library.load();
		void call(commands.listAllImages(), { ...src, silent: true })
			.then((list) => (presetImages = list.filter((i) => i.tags.includes("preset"))))
			.catch(() => {});
	});

	const saved = $derived((pluginSetting.value?.imageGen ?? null) as ImageGenSetting | null);
	const profile = $derived(library.apis.find((a) => a.id === saved?.apiProfileId));
	const modes = $derived([{ id: "", name: $_("phoneExtra.camDefault") }, ...presetImages.map((i) => ({ id: i.path, name: i.name || i.prompt.slice(0, 12) || $_("phoneExtra.camPreset") }))]);
	/** 고른 모드의 프리셋 (기본이면 null → 설정 값 그대로) */
	const preset = $derived.by(() => {
		const img = mode ? presetImages.find((i) => i.path === mode) : null;
		return img && saved ? { ...saved, ...presetFromMeta(img.meta) } : null;
	});
	const ready = $derived(!!prompt.trim() && !!profile && !shooting);

	async function shoot() {
		if (!ready) return;
		shooting = true;
		try {
			const img = await call(commands.generateImageFromPrompt(prompt, ghost, "phone", preset, album || null), src);
			shots = [img, ...shots];
			current = img;
		} finally {
			shooting = false;
		}
	}
</script>

<div class="cam">
	<div class="top">
		<button class="icon" aria-label={$_("phoneExtra.backAria")} onclick={onback}><ChevronLeft /></button>
		<div class="modes" role="radiogroup" aria-label={$_("phoneExtra.camPreset")}>
			{#each modes as m (m.id)}
				<button role="radio" aria-checked={mode === m.id} class:on={mode === m.id} onclick={() => (mode = m.id)}>{m.name}</button>
			{/each}
		</div>
		<button class="icon" aria-label={$_("phoneExtra.camPresetSettingsAria")} onclick={() => nav.go({ page: "settings", tab: "imagegen" })}><Settings /></button>
	</div>

	<div class="finder">
		{#if shooting}
			<div class="msg"><Spinner /><span>{$_("phoneExtra.camShooting")}</span></div>
		{:else if current}
			<img src={fileSrc(current.path)} alt={current.prompt} />
		{:else if !profile}
			<div class="msg">{$_("phoneExtra.camNoApi")}<button class="link" onclick={() => nav.go({ page: "settings", tab: "imagegen" })}>{$_("phoneExtra.camPickApi")}</button></div>
		{/if}
	</div>

	<div class="scene">
		<div class="how" role="radiogroup" aria-label={$_("phoneExtra.camSceneModeAria")}>
			<button role="radio" aria-checked={!ghost} class:on={!ghost} onclick={() => (ghost = false)}>{$_("phoneExtra.camDirect")}</button>
			<button role="radio" aria-checked={ghost} class:on={ghost} onclick={() => (ghost = true)}>{$_("phoneExtra.camGhost")}</button>
		</div>
		<textarea bind:value={prompt} rows="2" aria-label={$_("phoneExtra.camSceneAria")} placeholder={ghost ? $_("phoneExtra.camGhostPlaceholder") : $_("phoneExtra.camDirectPlaceholder")}></textarea>
		{#if ghost}<small>{$_("phoneExtra.camGhostNote")}</small>{/if}
	</div>

	<div class="controls">
		<button class="last" aria-label={$_("phoneExtra.camViewPhotosAria")} onclick={onalbum}>
			{#if shots[0]}<img src={fileSrc(shots[0].path)} alt="" />{/if}
		</button>
		<button class="shutter" aria-label={$_("phoneExtra.camShootAria")} disabled={!ready} onclick={shoot}><span></span></button>
		<span class="last ghost-slot" aria-hidden="true"></span>
	</div>
</div>

<style>
	.cam {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: #000;
		color: #fff;
	}
	.top {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 10px 8px;
	}
	.icon {
		display: grid;
		place-items: center;
		padding: 4px;
		border: none;
		background: none;
		color: inherit;
		cursor: pointer;
	}
	.icon :global(svg) {
		width: 22px;
		height: 22px;
	}
	.modes {
		flex: 1;
		display: flex;
		gap: 14px;
		overflow-x: auto;
		scrollbar-width: none;
		justify-content: safe center;
	}
	.modes button,
	.how button {
		flex: 0 0 auto;
		padding: 2px 0;
		border: none;
		background: none;
		color: rgb(255 255 255 / 0.6);
		font-size: 12px;
		font-weight: 600;
		letter-spacing: 0.04em;
		cursor: pointer;
	}
	.modes button.on,
	.how button.on {
		color: #ffd60a;
	}
	.finder {
		flex: 1;
		min-height: 0;
		display: grid;
		place-items: center;
		background: #111;
		overflow: hidden;
	}
	.finder img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.msg {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		font-size: 13px;
		color: rgb(255 255 255 / 0.7);
	}
	.link {
		border: none;
		background: none;
		color: #ffd60a;
		font: inherit;
		cursor: pointer;
	}
	.scene {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px 14px 0;
	}
	.how {
		display: flex;
		justify-content: center;
		gap: 16px;
	}
	.scene textarea {
		resize: none;
		padding: 8px 12px;
		border: 1px solid rgb(255 255 255 / 0.2);
		border-radius: 14px;
		background: rgb(255 255 255 / 0.08);
		color: #fff;
		font: inherit;
		font-size: 13px;
	}
	.scene small {
		text-align: center;
		font-size: 11px;
		color: rgb(255 255 255 / 0.55);
	}
	.controls {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 28px 18px;
	}
	.last {
		width: 44px;
		height: 44px;
		padding: 0;
		border: 2px solid rgb(255 255 255 / 0.5);
		border-radius: 10px;
		background: #222;
		overflow: hidden;
		cursor: pointer;
	}
	.last img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.ghost-slot {
		visibility: hidden;
	}
	.shutter {
		display: grid;
		place-items: center;
		width: 66px;
		height: 66px;
		padding: 0;
		border: 4px solid #fff;
		border-radius: 50%;
		background: none;
		cursor: pointer;
	}
	.shutter span {
		width: 52px;
		height: 52px;
		border-radius: 50%;
		background: #fff;
		transition: transform 0.1s;
	}
	.shutter:active:not(:disabled) span {
		transform: scale(0.9);
	}
	.shutter:disabled {
		opacity: 0.4;
		cursor: default;
	}
</style>
