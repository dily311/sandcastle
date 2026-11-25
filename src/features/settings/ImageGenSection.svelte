<script lang="ts">
	import UtilMacroNote from "$features/character/UtilMacroNote.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { jobs } from "$shared/stores/jobs.svelte";
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
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
		await addPresetImages(await pickFiles([{ name: $_("settings.more.picFilter"), extensions: ["png", "jpg", "jpeg", "webp"] }], true));
	}
	$effect(() =>
		dropPaths({ label: $_("settings.more2.dropPresetImgLabel"), hint: $_("settings.more2.dropPresetImgHint"), accept: (f) => isImage(f) && extOf(f) !== "gif", onpaths: addPresetImages }),
	);
	async function addPresetImages(paths: string[]) {
		if (!paths.length) return;
		const tagged = await jobs.run(
			$_("settings.more2.uploadImgJob", { values: { count: paths.length } }),
			async () => {
				const added = await call(commands.importGalleryImages(paths), { source: "imagegen" });
				const out: GeneratedImage[] = [];
				for (const a of added) out.push(await call(commands.setImageTags(a.path, [...a.tags, "preset"]), { source: "imagegen" }));
				return out;
			},
			{ group: $_("settings.more2.uploadImgGroup") },
		);
		if (!tagged) return;
		presetImages = [...tagged, ...presetImages];
		const withInfo = tagged.find((t) => Object.keys(presetFromMeta(t.meta)).length);
		if (withInfo) applyPreset(withInfo);
		else toasts.info($_("settings.more2.noGenMetaToast"));
	}
	function applyPreset(img: GeneratedImage) {
		const p = presetFromMeta(img.meta);
		if (!Object.keys(p).length) return;
		save("imageGen", p);
		toasts.success($_("settings.more2.appliedGenMetaToast", { values: { name: img.name || $_("settings.more.picFilter") } }));
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

<Section title={$_("settings.imageGenSec.title")} lead={$_("settings.imageGenSec.lead")}>
	{#if cur}
		<Group title={$_("settings.imageGenSec.apiGroup")}>
			<Row label={$_("settings.imageGenSec.api")} desc={$_("settings.imageGenSec.apiDesc")}>
				<Select value={cur.imageGen.apiProfileId ?? ""} options={apiOptions($_("settings.chatSec.notSelected"), ["novelai", "openai"])} onchange={(e) => save("imageGen", { apiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			<Row label={$_("settings.imageGenSec.model")}>
				{#if MODELS.includes(cur.imageGen.model)}
					<Select value={cur.imageGen.model} options={opts(MODELS)} onchange={(e) => save("imageGen", { model: e.currentTarget.value })} />
				{:else}
					<div class="pair">
						<Input value={cur.imageGen.model} onchange={(e) => save("imageGen", { model: e.currentTarget.value.trim() })} />
						<Button size="sm" variant="ghost" onclick={() => save("imageGen", { model: MODELS[0] })}>{$_("common.default")}</Button>
					</div>
				{/if}
			</Row>
			<Row label={$_("settings.imageGenSec.size")}>
				<Select value={cur.imageGen.size} options={opts(SIZES.includes(cur.imageGen.size) ? SIZES : [cur.imageGen.size, ...SIZES])} onchange={(e) => save("imageGen", { size: e.currentTarget.value })} />
			</Row>
			<Row label={$_("settings.imageGenSec.promptApi")} desc={$_("settings.imageGenSec.promptApiDesc")}>
				<Select value={cur.imageGen.promptApiProfileId ?? ""} options={chatApiOptions($_("settings.summarySec.chatApi"))} onchange={(e) => save("imageGen", { promptApiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			{#if cur.imageGen.promptApiProfileId}
				<Row label={$_("settings.imageGenSec.promptModel")} desc={$_("settings.summarySec.modelDesc")}>
					<ModelPicker profile={library.apis.find((a) => a.id === cur.imageGen.promptApiProfileId)} value={cur.imageGen.promptModel || null} onchange={(m) => save("imageGen", { promptModel: m ?? "" })} label={$_("settings.imageGenSec.promptModel")} />
				</Row>
			{/if}
			<Row label={$_("settings.imageGenSec.aiPrompt")} desc={$_("settings.imageGenSec.aiPromptDesc")} nowrap>
				<Switch checked={cur.imageGen.aiPrompt} label={$_("settings.imageGenSec.aiPrompt")} hideLabel onchange={(v) => save("imageGen", { aiPrompt: v })} />
			</Row>
		</Group>

		<Group title={$_("settings.imageGenSec.presetGroup")}>
			<Row label={$_("settings.imageGenSec.presetRow")} desc={$_("settings.imageGenSec.presetRowDesc")}>
				<div class="pair">
					<Button size="sm" onclick={uploadPreset}>{#snippet icon()}<Upload />{/snippet}{$_("settings.imageGenSec.uploadImage")}</Button>
					<Button size="sm" variant="ghost" onclick={() => nav.go({ page: "images" })}>{#snippet icon()}<Images />{/snippet}{$_("common.create")}</Button>
				</div>
			</Row>
			{#if nai}
				<Row label={$_("settings.imageGenSec.quality")} desc={$_("settings.imageGenSec.qualityDesc")}>
					<Select value={cur.imageGen.quality ?? "standard"} options={[{ value: "standard", label: $_("common.default") }, { value: "light", label: $_("settings.imageGenSec.qualityLight") }, { value: "none", label: $_("common.none") }]} onchange={(e) => save("imageGen", { quality: e.currentTarget.value })} />
				</Row>
			{/if}
			<div class="presets">
				{#if presetImages.length}
					<div class="pgrid">
						{#each presetImages as img (img.path)}
							{@const p = presetFromMeta(img.meta)}
							<button class="pcard" title={img.prompt} disabled={!Object.keys(p).length} onclick={() => applyPreset(img)}>
								<img src={fileSrc(img.path)} alt={img.name ?? img.prompt} loading="lazy" />
								<span class="pname">{img.name || img.prompt || $_("settings.more.picFilter")}</span>
								<span class="psum">{presetSummary(p) || $_("settings.more2.noGenMeta")}</span>
							</button>
						{/each}
					</div>
				{:else}
					<p class="pdesc">{$_("settings.more2.noPresetsHint")}</p>
				{/if}
			</div>
		</Group>

		{#if plugins.on("imageauto")}
			<Group title={$_("settings.more2.autoImageGroup")}>
				<UtilMacroNote presetId={appSetting.value?.selectedPreset ?? null} macro="auto_image" />
				<PromptSlotEditor key="autoImage" desc={$_("settings.more2.autoImageDesc")} vars={$_("settings.more2.autoImageVars")} />
			</Group>
		{/if}
		<Group title={$_("settings.more2.imagePromptGroup")}>
			<PromptSlotEditor key={nai ? "imagePromptNai" : "imagePrompt"} desc={nai ? $_("settings.more2.imagePromptNaiDesc") : $_("settings.more2.imagePromptDesc")} />
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
