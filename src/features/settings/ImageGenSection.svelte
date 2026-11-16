<script lang="ts">
	import { onMount } from "svelte";
	import { Images } from "@lucide/svelte";
	import { Button, Input, Select, Slider, Switch, Textarea } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
	import { library } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import { apiOptions, chatApiOptions, current, ensureLoaded, orNull, save } from "./pluginSettingForm.svelte";

	onMount(ensureLoaded);
	const cur = $derived(current());
	const profile = $derived(library.apis.find((a) => a.id === cur?.imageGen.apiProfileId));
	const nai = $derived(profile?.formatType === "novelai");

	const MODELS = $derived(
		nai ? ["nai-diffusion-4-5-full", "nai-diffusion-4-5-curated", "nai-diffusion-4-full", "nai-diffusion-4-curated-preview", "nai-diffusion-3"] : ["gpt-image-1", "dall-e-3", "dall-e-2"],
	);
	const SIZES = $derived(nai ? ["832x1216", "1216x832", "1024x1024", "640x640", "512x768", "768x512"] : ["1024x1024", "1024x1536", "1536x1024", "512x512", "auto"]);
	const SAMPLERS = ["k_euler_ancestral", "k_euler", "k_dpmpp_2s_ancestral", "k_dpmpp_2m_sde", "k_dpmpp_2m", "k_dpmpp_sde"];
	const opts = (list: string[]) => list.map((v) => ({ value: v, label: v }));
</script>

<Section title="이미지 생성" lead="대화 장면이나 직접 쓴 프롬프트로 그림을 만듦. NovelAI 또는 OpenAI 호환 이미지 API">
	{#snippet actions()}
		<Button size="sm" onclick={() => nav.go({ page: "images" })}>{#snippet icon()}<Images />{/snippet}이미지 페이지</Button>
	{/snippet}
	{#if cur}
		<Group title="기본 API">
			<Row label="이미지 API" desc="API 키 화면에서 'NovelAI (이미지)' 연결을 만들어 고름">
				<Select value={cur.imageGen.apiProfileId ?? ""} options={apiOptions("고르기", ["novelai", "openai"])} onchange={(e) => save("imageGen", { apiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			<Row label="모델">
				{#if MODELS.includes(cur.imageGen.model)}
					<Select value={cur.imageGen.model} options={opts(MODELS)} onchange={(e) => save("imageGen", { model: e.currentTarget.value })} />
				{:else}
					<div class="pair">
						<Input value={cur.imageGen.model} onchange={(e) => save("imageGen", { model: e.currentTarget.value.trim() })} />
						<Button size="sm" variant="ghost" onclick={() => save("imageGen", { model: MODELS[0] })}>기본값</Button>
					</div>
				{/if}
			</Row>
			<Row label="크기">
				<Select value={cur.imageGen.size} options={opts(SIZES.includes(cur.imageGen.size) ? SIZES : [cur.imageGen.size, ...SIZES])} onchange={(e) => save("imageGen", { size: e.currentTarget.value })} />
			</Row>
			<Row label="그림 프롬프트를 쓸 API" desc="메시지로 그릴 때 장면을 프롬프트로 정리하는 채팅 AI (NovelAI면 태그로 씀)">
				<Select value={cur.imageGen.promptApiProfileId ?? ""} options={chatApiOptions("대화에 고른 API")} onchange={(e) => save("imageGen", { promptApiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			<Row label="AI가 그림 프롬프트 쓰기" desc="끄면 메시지 글을 그대로 프롬프트로 보냄">
				<Switch checked={cur.imageGen.aiPrompt} label="AI 프롬프트" hideLabel onchange={(v) => save("imageGen", { aiPrompt: v })} />
			</Row>
		</Group>

		<Group title="생성 프리셋">
			<Row label="화풍·품질 태그" desc="모든 그림 프롬프트 끝에 붙음" stacked>
				<Textarea value={cur.imageGen.stylePrompt} rows={2} autosize maxRows={6} placeholder={nai ? "masterpiece, best quality, very aesthetic" : "anime style, soft lighting"} onchange={(e) => save("imageGen", { stylePrompt: e.currentTarget.value })} />
			</Row>
			{#if nai || !profile}
				<Row label="네거티브 프롬프트" desc="NovelAI: 그리지 않을 것" stacked>
					<Textarea value={cur.imageGen.negativePrompt} rows={2} autosize maxRows={6} onchange={(e) => save("imageGen", { negativePrompt: e.currentTarget.value })} />
				</Row>
				<div class="slider">
					<Slider label="단계 (steps)" hint="많을수록 정교하지만 느리고 비쌈 (Opus 무료는 28까지)" bind:value={() => String(cur.imageGen.steps), (v) => save("imageGen", { steps: Math.round(Number(v)) || 28 })} min={1} max={50} step={1} fallback={28} format={(n) => String(n)} />
				</div>
				<div class="slider">
					<Slider label="프롬프트 반영 (scale)" bind:value={() => String(cur.imageGen.scale), (v) => save("imageGen", { scale: Number(v) || 5 })} min={1} max={10} step={0.1} fallback={5} format={(n) => n.toFixed(1)} />
				</div>
				<Row label="샘플러">
					<Select value={cur.imageGen.sampler} options={opts(SAMPLERS)} onchange={(e) => save("imageGen", { sampler: e.currentTarget.value })} />
				</Row>
			{/if}
		</Group>
	{/if}
</Section>

<style>
	.pair {
		display: flex;
		gap: var(--space-2);
		align-items: center;
	}
	.slider {
		padding: var(--space-2) var(--space-3);
	}
</style>
