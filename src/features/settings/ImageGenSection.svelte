<script lang="ts">
	import { jobs } from "$shared/stores/jobs.svelte";
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	import { onMount } from "svelte";
	import { Images } from "@lucide/svelte";
	import { Button, Input, Select, Switch, Textarea } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
	import { library } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import PromptSlotEditor from "./PromptSlotEditor.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import type { GeneratedImage_Serialize as GeneratedImage } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { fileSrc } from "$shared/ipc/file";
	import { toasts } from "$shared/stores/toast.svelte";
	import { presetFromMeta, presetSummary } from "$features/images/presetFromImage";
	import { pickFiles } from "$shared/ipc/dialog";
	import { dropPaths, extOf, isImage } from "$app/drop.svelte";
	import { Upload } from "@lucide/svelte";
	import { apiOptions, chatApiOptions, current, ensureLoaded, orNull, save } from "./pluginSettingForm.svelte";

	// 프리셋 태그를 단 그림 (화면이 열릴 때 한 번 읽음)
	let presetImages = $state<GeneratedImage[]>([]);
	onMount(() => {
		ensureLoaded();
		void call(commands.listAllImages(), { source: "imagegen", silent: true })
			.then((list) => (presetImages = list.filter((i) => i.tags.includes("preset"))))
			.catch(() => {});
	});
	/** AI 그림을 올려 #프리셋 태그로 갤러리에 넣고, 생성 정보가 있으면 바로 설정에 */
	async function uploadPreset() {
		await addPresetImages(await pickFiles([{ name: "그림", extensions: ["png", "jpg", "jpeg", "webp"] }], true));
	}
	$effect(() =>
		dropPaths({ label: "프리셋 그림으로 올리기", hint: "PNG·JPG·WEBP (AI 그림이면 생성 정보를 설정에 넣음)", accept: (f) => isImage(f) && extOf(f) !== "gif", onpaths: addPresetImages }),
	);
	async function addPresetImages(paths: string[]) {
		if (!paths.length) return;
		const tagged = await jobs.run(
			`그림 ${paths.length}장`,
			async () => {
				const added = await call(commands.importGalleryImages(paths), { source: "imagegen" });
				const out: GeneratedImage[] = [];
				for (const a of added) out.push(await call(commands.setImageTags(a.path, [...a.tags, "preset"]), { source: "imagegen" }));
				return out;
			},
			{ group: "그림 올리기" },
		);
		if (!tagged) return;
		presetImages = [...tagged, ...presetImages];
		const withInfo = tagged.find((t) => Object.keys(presetFromMeta(t.meta)).length);
		if (withInfo) applyPreset(withInfo);
		else toasts.info("생성 정보가 없는 그림이라 프리셋 목록에만 넣음");
	}
	function applyPreset(img: GeneratedImage) {
		const p = presetFromMeta(img.meta);
		if (!Object.keys(p).length) return;
		save("imageGen", p);
		toasts.success(`'${img.name || "그림"}'의 생성 정보를 넣음`);
	}
	const cur = $derived(current());
	const profile = $derived(library.apis.find((a) => a.id === cur?.imageGen.apiProfileId));
	const nai = $derived(profile?.formatType === "novelai");

	const MODELS = $derived(
		nai ? ["nai-diffusion-4-5-full", "nai-diffusion-4-5-curated", "nai-diffusion-4-full", "nai-diffusion-4-curated-preview", "nai-diffusion-3"] : ["gpt-image-1", "dall-e-3", "dall-e-2"],
	);
	const SIZES = $derived(nai ? ["832x1216", "1216x832", "1024x1024", "640x640", "512x768", "768x512"] : ["1024x1024", "1024x1536", "1536x1024", "512x512", "auto"]);
	const opts = (list: string[]) => list.map((v) => ({ value: v, label: v }));
</script>

<Section title="이미지 생성" lead="대화 장면이나 직접 쓴 프롬프트로 그림을 만듦. NovelAI 또는 OpenAI 호환 이미지 API">
	{#if cur}
		<Group title="기본 API">
			<Row label="이미지 API" desc="API 키 화면에서 'NovelAI (이미지)' 연결을 만들어 고름">
				<Select value={cur.imageGen.apiProfileId ?? ""} options={apiOptions("미선택", ["novelai", "openai"])} onchange={(e) => save("imageGen", { apiProfileId: orNull(e.currentTarget.value) })} />
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
			{#if cur.imageGen.promptApiProfileId}
				<Row label="그림 프롬프트 모델" desc="비우면 그 API의 기본 모델">
					<ModelPicker profile={library.apis.find((a) => a.id === cur.imageGen.promptApiProfileId)} value={cur.imageGen.promptModel || null} onchange={(m) => save("imageGen", { promptModel: m ?? "" })} label="그림 프롬프트 모델" />
				</Row>
			{/if}
			<Row label="AI가 그림 프롬프트 쓰기" desc="끄면 메시지 글을 그대로 프롬프트로 보냄" nowrap>
				<Switch checked={cur.imageGen.aiPrompt} label="AI 프롬프트" hideLabel onchange={(v) => save("imageGen", { aiPrompt: v })} />
			</Row>
		</Group>

		<Group title="생성 프리셋">
			<Row label="네거티브·단계·샘플러 등" desc="AI 그림을 올리면 그 생성 정보로 프리셋을 만듦 (#프리셋 태그로 갤러리에 들어감). 그림을 보며 고치려면 이미지 페이지의 새로 만들기">
				<div class="pair">
					<Button size="sm" onclick={uploadPreset}>{#snippet icon()}<Upload />{/snippet}그림 올리기</Button>
					<Button size="sm" variant="ghost" onclick={() => nav.go({ page: "images" })}>{#snippet icon()}<Images />{/snippet}새로 만들기</Button>
				</div>
			</Row>
			{#if nai}
				<Row label="품질 태그" desc="프롬프트 끝에 붙는 NovelAI 품질 태그">
					<Select value={cur.imageGen.quality ?? "standard"} options={[{ value: "standard", label: "기본" }, { value: "light", label: "가볍게" }, { value: "none", label: "없음" }]} onchange={(e) => save("imageGen", { quality: e.currentTarget.value })} />
				</Row>
			{/if}
			<div class="presets">
				{#if presetImages.length}
					<div class="pgrid">
						{#each presetImages as img (img.path)}
							{@const p = presetFromMeta(img.meta)}
							<button class="pcard" title={img.prompt} disabled={!Object.keys(p).length} onclick={() => applyPreset(img)}>
								<img src={fileSrc(img.path)} alt={img.name ?? img.prompt} loading="lazy" />
								<span class="pname">{img.name || img.prompt || "그림"}</span>
								<span class="psum">{presetSummary(p) || "생성 정보 없음"}</span>
							</button>
						{/each}
					</div>
				{:else}
					<p class="pdesc">아직 없음 — 이미지 페이지에서 그림을 열고 #프리셋을 누름</p>
				{/if}
			</div>
		</Group>

		{#if plugins.on("imageauto")}
			<Group title="이미지 자동 생성 프롬프트">
				<PromptSlotEditor key="autoImage" desc="글을 쓰는 AI에게 응답 끝에 <image>그림 프롬프트</image>를 쓰게 하는 지시. 응답이 오면 이 프롬프트로 바로 그림 (응답마다 이미지 요금)" />
			</Group>
		{/if}
		<Group title="그림 프롬프트 쓰기">
			<PromptSlotEditor key={nai ? "imagePromptNai" : "imagePrompt"} desc={nai ? "NovelAI: 장면을 Danbooru 태그로 정리하는 지시" : "장면을 영어 그림 프롬프트로 정리하는 지시"} />
		</Group>
	{/if}
</Section>

<style>
	.presets {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-4) 0;
	}
	.pdesc {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.pgrid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
		gap: var(--space-2);
	}
	.pcard {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 0 0 var(--space-2);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		text-align: left;
		overflow: hidden;
		cursor: pointer;
	}
	.pcard:hover:not(:disabled) {
		border-color: var(--accent);
	}
	.pcard:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.pcard img {
		width: 100%;
		aspect-ratio: 1 / 1;
		object-fit: cover;
	}
	.pname,
	.psum {
		padding: 0 var(--space-2);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--text-xs);
	}
	.psum {
		color: var(--text-muted);
	}
	.pair {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		/* 좁은 화면: 입력칸이 줄어들고 버튼은 칸 밖으로 나가지 않음 */
		max-width: 100%;
	}
	.pair > :global(input),
	.pair > :global(.input) {
		min-width: 0;
	}
</style>
