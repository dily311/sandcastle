<script lang="ts">
	// 그림 만들기 (이미지 페이지의 "만들기"·폰 카메라 앱이 같이 씀). 왼쪽: 프롬프트·생성 프리셋, 오른쪽: 만든 그림.
	// 프리셋은 설정 값에서 시작하고, 여기서 고치면 이번 그리기에만 쓴다 ("설정에 저장"을 눌러야 저장)
	import { onMount } from "svelte";
	import { RotateCcw, Save, Wand2 } from "@lucide/svelte";
	import type { GeneratedImage_Serialize as GeneratedImage, ImageGenSetting_Serialize as ImageGenSetting } from "$ipc/bindings";
	import { Button, Input, ProfilePicker, Select, Slider, Spinner, Switch, Textarea, type ProfileOption } from "$shared/ui";
	import { presetFromMeta, presetSummary } from "./presetFromImage";
	import { call, commands } from "$shared/ipc/client";
	import { fileSrc } from "$shared/ipc/file";
	import { toasts } from "$shared/stores/toast.svelte";
	import { nav } from "$app/nav.svelte";
	import { library } from "$features/character/library.svelte";
	import { pluginSetting } from "$shared/stores/aiTools.svelte";

	interface Props {
		/** 갤러리 출처: manual(이미지 페이지) / phone(폰 카메라) */
		source?: "manual" | "phone";
		/** 처음 프롬프트 (갤러리의 "이 프롬프트로 다시") */
		initialPrompt?: string;
		/** 폰 카메라: 그 폰 주인 앨범 ("persona:<id>") */
		album?: string;
		/** 좁은 곳(폰 화면): 한 줄로 쌓고 그림을 위에 */
		compact?: boolean;
		/** 그림이 만들어지면 */
		onmade?: (img: GeneratedImage) => void;
		/** 만든 그림을 누르면 (없으면 크게 보기 없이) */
		onview?: (img: GeneratedImage) => void;
	}
	let { source = "manual", initialPrompt = "", album, compact = false, onmade, onview }: Props = $props();
	const src = { source: "imagegen" };

	// #프리셋 태그를 단 그림 (고르면 그 생성 정보를 이번 프리셋에)
	let presetImages = $state<GeneratedImage[]>([]);
	let presetPick = $state<string | null>(null);
	onMount(() => {
		if (!pluginSetting.value) void pluginSetting.load();
		if (!library.loaded) void library.load();
		void call(commands.listAllImages(), { source: "imagegen", silent: true })
			.then((list) => (presetImages = list.filter((i) => i.tags.includes("preset"))))
			.catch(() => {});
	});
	const presetOptions = $derived<ProfileOption[]>([
		{ id: "", name: "설정의 프리셋", description: "설정 > 이미지 생성 값 그대로" },
		...presetImages.map((i) => ({ id: i.path, name: i.name || i.prompt.slice(0, 30) || "그림", avatar: i.path, description: presetSummary(presetFromMeta(i.meta)) || "생성 정보 없음" })),
	]);
	function pickPresetImage(id: string) {
		presetPick = id || null;
		if (!saved) return;
		if (!id) return void (draft = null);
		const img = presetImages.find((i) => i.path === id);
		if (img) draft = { ...saved, ...presetFromMeta(img.meta) };
	}

	// 처음 값만 쓴다 (다른 프롬프트로 다시 열 때는 부모가 {#key}로 새로 만든다)
	// svelte-ignore state_referenced_locally
	let prompt = $state(initialPrompt);
	let refine = $state(false);
	let making = $state(false);
	let made = $state<GeneratedImage[]>([]);
	let current = $state<GeneratedImage | null>(null);

	// 이번 화면에서 고친 프리셋 (null = 설정 값 그대로)
	let draft = $state<ImageGenSetting | null>(null);
	const saved = $derived((pluginSetting.value?.imageGen ?? null) as ImageGenSetting | null);
	const preset = $derived(draft ?? saved);
	const edited = $derived(!!draft && JSON.stringify(draft) !== JSON.stringify(saved));
	function edit<K extends keyof ImageGenSetting>(k: K, v: ImageGenSetting[K]) {
		if (!saved) return;
		draft = { ...(draft ?? saved), [k]: v };
	}
	async function savePreset() {
		if (!draft || !pluginSetting.value) return;
		await pluginSetting.save({ ...pluginSetting.value, imageGen: draft });
		draft = null;
		toasts.success("설정에 저장함");
	}

	const profile = $derived(library.apis.find((a) => a.id === preset?.apiProfileId));
	const nai = $derived(profile?.formatType === "novelai");
	// 주로 4.5·5를 쓴다 (5도 4 이후 형식으로 보냄, service/plugins.rs)
	const MODELS = $derived(nai ? ["nai-diffusion-4-5-full", "nai-diffusion-4-5-curated", "nai-diffusion-5-full", "nai-diffusion-5-curated", "nai-diffusion-4-full", "nai-diffusion-3"] : ["gpt-image-1", "dall-e-3", "dall-e-2"]);
	const QUALITY = [
		{ value: "standard", label: "기본 (very aesthetic, masterpiece, no text)" },
		{ value: "light", label: "가볍게 (very aesthetic, masterpiece)" },
		{ value: "none", label: "없음" },
	];
	const SIZES = $derived(nai ? ["832x1216", "1216x832", "1024x1024", "640x640", "512x768", "768x512"] : ["1024x1024", "1024x1536", "1536x1024", "512x512", "auto"]);
	const SAMPLERS = ["k_euler_ancestral", "k_euler", "k_dpmpp_2s_ancestral", "k_dpmpp_2m_sde", "k_dpmpp_2m", "k_dpmpp_sde"];
	const opts = (list: string[], cur?: string) => (cur && !list.includes(cur) ? [cur, ...list] : list).map((v) => ({ value: v, label: v }));

	async function make() {
		if (!prompt.trim() || making) return;
		making = true;
		try {
			const img = await call(commands.generateImageFromPrompt(prompt, refine, source, edited ? draft : null, album ?? null), src);
			made = [img, ...made];
			current = img;
			onmade?.(img);
		} finally {
			making = false;
		}
	}
</script>

<div class="create" class:compact>
	<section class="left">
		<p class="api">
			{#if profile}{profile.name}{:else}이미지 API가 없음 — <button class="link" onclick={() => nav.go({ page: "settings", tab: "imagegen" })}>설정에서 고르기</button>{/if}
		</p>
		{#if !preset}
			<label class="field"><span>프롬프트</span><Textarea bind:value={prompt} rows={5} autosize maxRows={14} placeholder="그릴 장면" aria-label="그릴 프롬프트" /></label>
		{/if}
		{#if preset}
			<div class="preset">
				<div class="ph">
					<b>생성 프리셋</b>
					{#if edited}
						<span class="dirty">이번만 바꿈</span>
						<Button size="sm" variant="ghost" onclick={() => ((draft = null), (presetPick = null))}>{#snippet icon()}<RotateCcw />{/snippet}되돌리기</Button>
						<Button size="sm" onclick={savePreset}>{#snippet icon()}<Save />{/snippet}설정에 저장</Button>
					{/if}
				</div>
				{#if presetImages.length}
					<div class="field">
						<span>프리셋 그림 <small>#프리셋 태그를 단 그림의 생성 정보</small></span>
						<ProfilePicker options={presetOptions} value={presetPick ?? ""} label="프리셋 그림 고르기" onchange={pickPresetImage} />
					</div>
				{/if}
				<label class="field"><span>프롬프트</span>
					<Textarea bind:value={prompt} rows={4} autosize maxRows={14} placeholder="그릴 장면. NovelAI는 태그(1girl, silver hair, forest, night…)가 잘 먹힘" aria-label="그릴 프롬프트" />
				</label>
				<Switch bind:checked={refine} label="AI가 프롬프트 다듬기" description="짧게 써도 장면 프롬프트로 정리함 (채팅 API 요금)" />
				{#if nai || !profile}
					<label class="field"><span>네거티브 프롬프트</span>
						<Textarea value={preset.negativePrompt} rows={2} autosize maxRows={6} oninput={(e) => edit("negativePrompt", e.currentTarget.value)} />
					</label>
				{/if}
				<div class="two">
					<label class="field"><span>모델</span>
						{#if MODELS.includes(preset.model)}
							<Select value={preset.model} options={opts(MODELS)} onchange={(e) => edit("model", e.currentTarget.value)} />
						{:else}
							<Input value={preset.model} onchange={(e) => edit("model", e.currentTarget.value.trim())} />
						{/if}
					</label>
					<label class="field"><span>크기</span><Select value={preset.size} options={opts(SIZES, preset.size)} onchange={(e) => edit("size", e.currentTarget.value)} /></label>
				</div>
				{#if nai || !profile}
					<label class="field"><span>품질 태그 <small>프롬프트 끝에 붙음</small></span><Select value={preset.quality ?? "standard"} options={QUALITY} onchange={(e) => edit("quality", e.currentTarget.value)} /></label>
					<Slider label="단계 (steps)" hint="많을수록 정교하지만 느리고 비쌈 (Opus 무료는 28까지)" bind:value={() => String(preset.steps), (v) => edit("steps", Math.round(Number(v)) || 28)} min={1} max={50} step={1} fallback={28} format={(n) => String(n)} />
					<Slider label="프롬프트 반영 (scale)" bind:value={() => String(preset.scale), (v) => edit("scale", Number(v) || 5)} min={1} max={10} step={0.1} fallback={5} format={(n) => n.toFixed(1)} />
					<label class="field"><span>샘플러</span><Select value={preset.sampler} options={opts(SAMPLERS, preset.sampler)} onchange={(e) => edit("sampler", e.currentTarget.value)} /></label>
				{/if}
			</div>
		{/if}
		<div class="go">
			<Button variant="primary" loading={making} disabled={!prompt.trim() || !profile} onclick={make}>{#snippet icon()}<Wand2 />{/snippet}그리기</Button>
		</div>
	</section>

	<section class="right" aria-label="만든 그림">
		{#if making}
			<div class="stage"><Spinner /><span>그리는 중… (수십 초 걸릴 수 있음)</span></div>
		{:else if current}
			<button class="stage pic" onclick={() => current && onview?.(current)} title={current.prompt}><img src={fileSrc(current.path)} alt={current.prompt} /></button>
		{:else}
			<div class="stage empty">만든 그림이 여기 뜸</div>
		{/if}
		{#if made.length > 1}
			<div class="thumbs">
				{#each made as img (img.path)}
					<button class="thumb" class:on={current?.path === img.path} onclick={() => (current = img)}><img src={fileSrc(img.path)} alt={img.prompt} /></button>
				{/each}
			</div>
		{/if}
	</section>
</div>

<style>
	.create {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		gap: var(--space-5);
		align-items: start;
	}
	.left,
	.right {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		min-width: 0;
	}
	.right {
		position: sticky;
		top: 0;
	}
	.api {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
	.link {
		padding: 0;
		border: none;
		background: none;
		color: var(--accent-text);
		font: inherit;
		cursor: pointer;
	}
	.preset {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-4);
		border-radius: var(--radius-lg);
		background: var(--bg-sunken);
	}
	.ph {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	.ph b {
		flex: 1;
		font-size: var(--text-sm);
	}
	.dirty {
		font-size: var(--text-xs);
		color: var(--warning, var(--text-muted));
	}
	.two {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
		min-width: 0;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.field small {
		color: var(--text-faint);
	}
	.go {
		display: flex;
		justify-content: flex-end;
	}
	.stage {
		display: grid;
		place-items: center;
		gap: var(--space-2);
		min-height: 320px;
		padding: 0;
		border: 1px dashed var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--bg-sunken);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.stage.pic {
		border-style: solid;
		cursor: zoom-in;
		overflow: hidden;
	}
	.stage img {
		display: block;
		max-width: 100%;
		max-height: 70vh;
		object-fit: contain;
	}
	.thumbs {
		display: flex;
		gap: var(--space-2);
		overflow-x: auto;
	}
	.thumb {
		flex: 0 0 auto;
		width: 64px;
		height: 64px;
		padding: 0;
		border: none;
		border-radius: var(--radius-md);
		overflow: hidden;
		cursor: pointer;
		box-shadow: 0 0 0 1px var(--border);
	}
	.thumb.on {
		box-shadow: 0 0 0 2px var(--accent);
	}
	.thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	@media (max-width: 760px) {
		.create {
			grid-template-columns: 1fr;
		}
		.right {
			position: static;
		}
	}
	/* 폰 화면: 그림 칸을 위로, 한 줄 */
	.create.compact {
		grid-template-columns: 1fr;
		gap: var(--space-3);
	}
	.compact .right {
		position: static;
		order: -1;
	}
	.compact .stage {
		min-height: 200px;
	}
	.compact .two {
		grid-template-columns: 1fr;
	}
</style>
