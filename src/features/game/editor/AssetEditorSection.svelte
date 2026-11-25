<script lang="ts">
	import { _ } from "svelte-i18n";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	/** 지우기 전에 앱 확인 창으로 물어봄 */
	const ask = (message: string) => confirmAction({ title: $_("common.delete"), message, confirmLabel: $_("common.delete"), danger: true });
	import type { PackageData } from "$features/game/types";
	import { call, commands, toAppError } from "$shared/ipc/client";
	import { reportGameError } from "$features/game/errors";
	import type {
		GameAssetsInput,
		GameAudioAssetsInput,
		GameBackgroundAssetsInput,
		GameCgAssetsInput,
		GameCharacterAssetsInput,
		GameLocationsInput,
		GameTimePeriodsInput,
		GameWeatherTypesInput,
	} from "$features/game/types";
	import { editorAssetPreviewUrl } from "./assetPreview";
	import { open } from "@tauri-apps/plugin-dialog";
	import { Button } from "$shared/ui";
	import { dropPaths, isAudio, isFont, isImage } from "$app/drop.svelte";

	type Props = {
		assets: GameAssetsInput[];
		audioAssets: GameAudioAssetsInput[];
		characterAssets: GameCharacterAssetsInput[];
		cgAssets: GameCgAssetsInput[];
		backgroundAssets: GameBackgroundAssetsInput[];
		assetRoot: string;
		gameId: string;
		characters: { id: string | null; name: string }[];
		locations: GameLocationsInput[];
		timePeriods: GameTimePeriodsInput[];
		weatherTypes: GameWeatherTypesInput[];
	};
	let {
		assets = $bindable([]),
		audioAssets = $bindable([]),
		characterAssets = $bindable([]),
		cgAssets = $bindable([]),
		backgroundAssets = $bindable([]),
		assetRoot,
		gameId,
		characters,
		locations,
		timePeriods,
		weatherTypes,
	}: Props = $props();
	let filter = $state("all");
	let entries = $derived([
		...assets.map((asset, index) => ({ asset, type: "asset", index })),
		...audioAssets.map((asset, index) => ({ asset, type: "audio", index })),
		...characterAssets.map((asset, index) => ({ asset, type: "character", index })),
		...cgAssets.map((asset, index) => ({ asset, type: "cg", index })),
		...backgroundAssets.map((asset, index) => ({ asset, type: "background", index })),
	]);
	let filtered = $derived(filter === "all" ? entries : entries.filter((entry) => entry.type === filter));
	function add(type: string) {
		const common = {
			id: crypto.randomUUID(),
			gameId,
			key: `asset_${Date.now()}`,
			name: $_("gameEditor.newAssetName"),
			category: "other",
			kind: "other",
			filePath: "",
			settingsJson: null,
		};
		if (type === "audio")
			audioAssets.push({
				...common,
				category: "sfx",
				kind: "audio",
				tag: null,
				gameCharacterId: null,
				transcript: null,
			});
		else if (type === "character")
			characterAssets.push({
				...common,
				category: "character",
				kind: "image",
				gameCharacterId: null,
				usage: "standing",
				emotion: null,
			});
		else if (type === "cg")
			cgAssets.push({
				...common,
				category: "cg",
				kind: "image",
				gameCharacterId: null,
				description: null,
				prompt: null,
				negativePrompt: null,
				seed: null,
				generationParams: null,
				aiUsable: false,
			});
		else if (type === "background")
			backgroundAssets.push({
				...common,
				category: "background",
				kind: "image",
				locationId: null,
				timeId: null,
				weatherId: null,
			});
		else assets.push(common);
	}
	// ---- 끌어다 놓아 에셋 추가 ----
	// 파일마다 에셋 하나: 소리는 오디오, 그림은 지금 보는 종류(캐릭터·일러스트·배경, 그 외는 기타), 글꼴은 기타
	const lastOf = (type: string) =>
		(type === "audio" ? audioAssets : type === "character" ? characterAssets : type === "cg" ? cgAssets : type === "background" ? backgroundAssets : assets).at(-1);
	const stemOf = (path: string) => (path.split(/[\\/]/).pop() ?? "").replace(/\.[^.]+$/, "");
	function uniqueKey(base: string) {
		const used = new Set(entries.map((e) => e.asset.key));
		const stem = base.replace(/[^\p{L}\p{N}_-]+/gu, "_").slice(0, 40) || "asset";
		let key = stem;
		for (let n = 2; used.has(key); n++) key = `${stem}_${n}`;
		return key;
	}
	const IMAGE_TYPES = ["character", "cg", "background"];
	/** 그림을 '일러스트'·'배경' 종류로 놓으면 백엔드가 파일 내용으로 형식을 확인하고 바로 게임 폴더에 넣는다 (저장을 기다리지 않음) */
	const isImagePath = (p: string) => ["png", "jpg", "jpeg", "webp", "gif"].includes(p.split(".").pop()?.toLowerCase() ?? "");
	async function addChecked(paths: string[], category: "cg" | "background") {
		const added: string[] = [];
		let problems = 0;
		for (const path of paths) {
			try {
				const r = await call(commands.addGameAsset(gameId, path, { category, name: stemOf(path) || null }), { source: "game", silent: true });
				added.push(r.asset.id);
				problems += r.diagnostics.length;
			} catch (e) {
				reportGameError(e, gameId);
			}
		}
		if (!added.length) return;
		// 안 저장한 다른 편집은 그대로 두고, 방금 넣은 행만 서버에서 가져와 덧붙인다
		const fresh = (await call(commands.getPackageData(gameId), { source: "game" })) as unknown as PackageData;
		const have = new Set(entries.map((e) => e.asset.id).filter(Boolean));
		const pick = category === "cg" ? fresh.cgAssets : fresh.backgroundAssets;
		for (const a of pick) if (a.id && added.includes(a.id) && !have.has(a.id)) (category === "cg" ? cgAssets : backgroundAssets).push(a as never);
		toasts.success($_("gameEditor.addedToFolderToast", {
			values: {
				count: added.length,
				extra: problems ? $_("gameEditor.addedProblemsPart", { values: { count: problems } }) : "",
			},
		}));
	}
	function addDropped(all: string[]) {
		let paths = all;
		if ((filter === "cg" || filter === "background") && all.some(isImagePath)) {
			const images = all.filter(isImagePath);
			paths = all.filter((p) => !isImagePath(p));
			void addChecked(images, filter);
		}
		for (const path of paths) {
			const ext = path.split(".").pop()?.toLowerCase() ?? "";
			const audio = ["mp3", "wav", "ogg", "flac", "m4a", "aac"].includes(ext);
			const font = ["ttf", "otf", "woff", "woff2"].includes(ext);
			const type = audio ? "audio" : !font && IMAGE_TYPES.includes(filter) ? filter : "asset";
			add(type);
			const created = lastOf(type);
			if (!created) continue;
			created.filePath = path;
			created.name = stemOf(path) || created.name;
			created.key = uniqueKey(stemOf(path));
			if (type === "asset") created.kind = font ? "font" : audio ? "audio" : "image";
		}
		if (paths.length) toasts.success($_("gameEditor.addedDroppedToast", { values: { count: paths.length } }));
	}
	$effect(() =>
		dropPaths({
			label: $_("gameEditor.dropAssetsLabel"),
			hint: $_("gameEditor.dropAssetsHint", {
				values: {
					dest: IMAGE_TYPES.includes(filter)
						? $_("gameEditor.dropAssetsDestTyped", { values: { type: typeName(filter) } })
						: $_("gameEditor.dropAssetsDestOther"),
				},
			}),
			accept: (f) => isImage(f) || isAudio(f) || isFont(f),
			onpaths: addDropped,
		}),
	);
	async function remove(type: string, index: number) {
		if (!await ask($_("gameEditor.deleteAssetConfirm"))) return;
		const list = type === "audio" ? audioAssets : type === "character" ? characterAssets : type === "cg" ? cgAssets : type === "background" ? backgroundAssets : assets;
		const target = list[index];
		// 이미 게임 폴더에 있는 에셋은 백엔드가 지운다: 대본·표지·아이콘이 쓰고 있으면 거절하고, 그래도 지울지 다시 묻는다
		if (target?.filePath.startsWith("assets/")) {
			let force = false;
			for (;;) {
				try {
					const r = await call(commands.removeGameAsset(gameId, target.id ?? "", force), { source: "game", silent: true });
					if (r.diagnostics.length) toasts.info($_("gameEditor.removedProblemsToast", { values: { count: r.diagnostics.length } }));
					break;
				} catch (e) {
					const err = toAppError(e);
					const d = err.details as { reason?: string; references?: string[] } | undefined;
					if (d?.reason !== "assetInUse" || force) return void reportGameError(e, gameId);
					const refs = (d.references ?? []).join(", ");
					if (!(await confirmAction({ title: $_("gameEditor.assetInUseTitle"), message: $_("gameEditor.assetInUseMsg", { values: { refs } }), confirmLabel: $_("gameEditor.forceDeleteBtn"), danger: true }))) return;
					force = true;
				}
			}
		}
		if (type === "audio") audioAssets.splice(index, 1);
		else if (type === "character") characterAssets.splice(index, 1);
		else if (type === "cg") cgAssets.splice(index, 1);
		else if (type === "background") backgroundAssets.splice(index, 1);
		else assets.splice(index, 1);
	}
	async function chooseFile(entry: (typeof entries)[number]) {
		const kind = entry.asset.kind;
		const extensions =
			kind === "image"
				? ["png", "jpg", "jpeg", "webp", "gif", "bmp", "avif", "svg"]
				: kind === "audio"
					? ["mp3", "wav", "ogg", "flac", "m4a", "aac"]
					: kind === "font"
						? ["ttf", "otf", "woff", "woff2"]
						: undefined;
		const selected = await open({
			multiple: false,
			...(extensions ? { filters: [{ name: kind, extensions }] } : {}),
		});
		if (typeof selected === "string") entry.asset.filePath = selected;
	}
	function typeName(type: string) {
		return (
			(
				{
					asset: $_("gameEditor.assetTypeOther"),
					audio: $_("gameEditor.assetTypeAudio"),
					character: $_("gameEditor.assetTypeCharacter"),
					cg: $_("gameEditor.assetTypeCg"),
					background: $_("gameEditor.assetTypeBackground"),
				} as Record<string, string>
			)[type] || type
		);
	}
</script>

<section class="asset-manager">
	<header class="asset-toolbar">
		<div>
			<h3>{$_("gameEditor.allAssetsHeading")}</h3>
			<p>{$_("gameEditor.allAssetsDesc")}</p>
		</div>
		<div class="action-btn">
			<select aria-label={$_("gameEditor.assetKindFilterAria")} bind:value={filter}>
				<option value="all">{$_("gameEditor.viewAll")}</option>
				<option value="audio">{$_("gameEditor.assetTypeAudio")}</option>
				<option value="character">{$_("gameEditor.assetTypeCharacter")}</option>
				<option value="cg">{$_("gameEditor.assetTypeCg")}</option>
				<option value="background">{$_("gameEditor.assetTypeBackground")}</option>
				<option value="asset">{$_("gameEditor.assetTypeOther")}</option>
			</select>
			<button class="add-btn" onclick={() => add(filter === "all" ? "asset" : filter)}>{$_("gameEditor.addAssetBtn")}</button>
		</div>
	</header>
	<div class="asset-grid">
		{#each filtered as entry (`${entry.type}-${entry.asset.id || entry.index}`)}
			<article class="asset-card">
				<div class="asset-preview">
					{#if entry.asset.kind === "image"}
						<button
							class="preview-select"
							type="button"
							aria-label={$_("gameEditor.pickImageShort")}
							onclick={() => chooseFile(entry)}
						>
							{#if entry.asset.filePath}
								<img
									src={editorAssetPreviewUrl(assetRoot, entry.asset.filePath)}
									alt={entry.asset.name}
								/>
							{:else}
								<span>＋<small>{$_("gameEditor.pickImageShort")}</small> </span>
							{/if}
						</button>
					{:else}
						<span>
							{entry.asset.kind === "audio" ? "♫" : entry.asset.kind === "font" ? "Aa" : "◇"}
						</span>
					{/if}
					<small>
						{typeName(entry.type)}
					</small>
				</div>
				{#if entry.asset.kind === "audio" && entry.asset.filePath}
					<audio
						class="audio-preview"
						controls
						preload="none"
						src={editorAssetPreviewUrl(assetRoot, entry.asset.filePath)}
						aria-label={$_("gameEditor.audioPreviewAria", { values: { name: entry.asset.name } })}
					>
					</audio>
				{/if}
				<div class="asset-fields">
					{#if entry.type !== "character"}
						<div class="field">
							<label>{$_("gameEditor.nameLabel")}</label>
							<input bind:value={entry.asset.name} />
						</div>
					{/if}
					<div class="field">
						<label>{$_("gameEditor.keyLabel")}</label>
						<input bind:value={entry.asset.key} />
					</div>
					<Button variant="secondary" size="sm" onclick={() => chooseFile(entry)}>
						{entry.asset.filePath ? $_("gameEditor.changeFileBtn") : $_("gameEditor.chooseFileBtn")}
					</Button>
					{#if entry.type === "asset"}
						<div class="field-row">
							<div class="field">
								<label>{$_("gameEditor.kindLabel")}</label>
								<select bind:value={entry.asset.kind}>
									<option value="image">{$_("gameEditor.kindImage")}</option>
									<option value="audio">{$_("gameEditor.kindAudio")}</option>
									<option value="font">{$_("gameEditor.kindFont")}</option>
									<option value="other">{$_("gameEditor.kindOther")}</option>
								</select>
							</div>
							<div class="field">
								<label>{$_("gameEditor.categoryLabel")}</label>
								<input bind:value={entry.asset.category} />
							</div>
						</div>
					{:else if entry.type === "audio"}{@const audio = entry.asset as GameAudioAssetsInput}
						<div class="field-row">
							<div class="field">
								<label>{$_("gameEditor.audioPurposeLabel")}</label>
								<select bind:value={audio.category}>
									<option value="voice">{$_("gameEditor.audioVoice")}</option>
									<option value="sfx">{$_("gameEditor.audioSfx")}</option>
									<option value="bgm">{$_("gameEditor.audioBgm")}</option>
								</select>
							</div>
							<div class="field">
								<label>{$_("gameEditor.tagLabel")}</label>
								<input bind:value={audio.tag} />
							</div>
						</div>
						{#if audio.category === "voice"}
							<div class="field">
								<label>{$_("gameEditor.characterLabel")}</label>
								<select bind:value={audio.gameCharacterId}>
									<option value={null}>{$_("gameEditor.notSelected")}</option>
									{#each characters as character}
										<option value={character.id}>
											{character.name}
										</option>
									{/each}
								</select>
							</div>
							<div class="field">
								<label>{$_("gameEditor.transcriptLabel")}</label>
								<textarea bind:value={audio.transcript} rows="2"></textarea>
							</div>
						{/if}
					{:else if entry.type === "character"}{@const characterAsset =
							entry.asset as GameCharacterAssetsInput}
						<div class="field-row">
							<div class="field">
								<label>{$_("gameEditor.characterLabel")}</label>
								<select bind:value={characterAsset.gameCharacterId}>
									<option value={null}>{$_("gameEditor.notSelected")}</option>
									{#each characters as character}
										<option value={character.id}>
											{character.name}
										</option>
									{/each}
								</select>
							</div>
							<div class="field">
								<label>{$_("gameEditor.emotionLabel")}</label>
								<input bind:value={characterAsset.emotion} />
							</div>
						</div>
						<div class="field">
							<label>{$_("gameEditor.usagePlaceLabel")}</label>
							<select bind:value={characterAsset.usage}>
								<option value="standing">{$_("gameEditor.usageStanding")}</option>
								<option value="map">{$_("gameEditor.usageMapShort")}</option>
							</select>
						</div>
					{:else if entry.type === "cg"}{@const cg = entry.asset as GameCgAssetsInput}
						<div class="field">
							<label>{$_("gameEditor.characterLabel")}</label>
							<select bind:value={cg.gameCharacterId}>
								<option value={null}>{$_("gameEditor.notSelected")}</option>
								{#each characters as character}
									<option value={character.id}>
										{character.name}
									</option>
								{/each}
							</select>
						</div>
						<div class="field">
							<label>{$_("gameEditor.descLabel")}</label>
							<textarea bind:value={cg.description} rows="2"></textarea>
						</div>
						<label class="check"> <input type="checkbox" bind:checked={cg.aiUsable} /> {$_("gameEditor.aiUsableLabel")}</label>
					{:else if entry.type === "background"}{@const background = entry.asset as GameBackgroundAssetsInput}
						<div class="field">
							<label>{$_("gameEditor.locationLabel")}</label>
							<select bind:value={background.locationId}>
								<option value={null}>{$_("gameEditor.notSelected")}</option>
								{#each locations as location (location.id)}
									<option value={location.id}>
										{location.name}
									</option>
								{/each}
							</select>
						</div>
						<div class="field-row">
							<div class="field">
								<label>{$_("gameEditor.tabTime")}</label>
								<select bind:value={background.timeId}>
									<option value={null}>{$_("gameEditor.allTimePeriods")}</option>
									{#each timePeriods as period (period.id)}
										<option value={period.id}>
											{period.name}
										</option>
									{/each}
								</select>
							</div>
							<div class="field">
								<label>{$_("gameEditor.tabWeather")}</label>
								<select bind:value={background.weatherId}>
									<option value={null}>{$_("gameEditor.allWeathers")}</option>
									{#each weatherTypes as weather (weather.id)}
										<option value={weather.id}>
											{weather.name}
										</option>
									{/each}
								</select>
							</div>
						</div>
					{/if}
				</div>
				<div class="delete-action">
					<Button variant="danger" size="sm" onclick={() => remove(entry.type, entry.index)}>{$_("gameEditor.deleteAssetBtn")}</Button
					>
				</div>
			</article>
		{:else}
			<div class="empty">{$_("gameEditor.noAssets")}</div>
		{/each}
	</div>
</section>

<style>
	.asset-manager {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		height: 100%;
		padding: var(--space-6);
		overflow: auto;
	}
	.asset-toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
	}
	.asset-toolbar h3 {
		margin: 0 0 0.25rem;
	}
	.asset-toolbar p {
		margin: 0;
		color: var(--text-muted);
		font-size: 0.85rem;
	}
	.action-btn {
		display: flex;
		gap: 0.5rem;
	}
	.asset-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 1rem;
	}
	.asset-card {
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-sm);
	}
	.asset-preview {
		height: 155px;
		max-height: 155px;
		overflow: hidden;
		background: var(--surface-2);
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
		font-size: 2rem;
	}
	.preview-select {
		place-items: center;
		width: 100%;
		height: 155px;
		max-height: 155px;
		overflow: hidden;
		padding: 0;
		border: 0;
		border-radius: var(--radius-md);
		background: var(--surface-active);
		color: var(--text-faint);
		font-size: 1.4rem;
		cursor: pointer;
	}
	.asset-preview img {
		width: 100%;
		height: 100%;
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.preview-select span {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
		font-size: 1.5rem;
	}
	.preview-select small {
		position: static;
		padding: 0;
		background: none;
		font-size: 0.72rem;
	}
	.audio-preview {
		display: block;
		width: calc(100% - 1.7rem);
		margin: 0.75rem 0.85rem 0;
		color-scheme: dark;
	}
	.asset-preview small {
		position: absolute;
		left: 0.6rem;
		top: 0.6rem;
		padding: 0.2rem 0.45rem;
		border-radius: var(--radius-sm);
		background: var(--bg);
		color: var(--text-muted);
		font-size: 0.68rem;
	}
	.asset-fields {
		padding: 0.85rem;
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		min-width: 0;
	}
	.field label {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
	.field input,
	.field select,
	.field textarea,
	.action-btn select {
		width: 100%;
		box-sizing: border-box;
		background: var(--bg);
		color: var(--text);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		padding: 0.55rem 0.65rem;
		font: inherit;
		font-size: 0.82rem;
	}
	.field-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.6rem;
	}
	.check {
		display: flex;
		gap: 0.4rem;
		align-items: center;
		color: var(--text-muted);
		font-size: 0.82rem;
	}
	.delete-action {
		display: flex;
		justify-content: flex-end;
		padding: 0 0.85rem 0.85rem;
	}
	.empty {
		grid-column: 1/-1;
		padding: 3rem;
		text-align: center;
		color: var(--text-muted);
	}
	@media (max-width: 650px) {
		.asset-toolbar {
			align-items: flex-start;
			flex-direction: column;
			width: 100%;
		}
		.action-btn {
			width: 100%;
		}
		.action-btn select {
			flex: 1;
		}
	}
</style>
