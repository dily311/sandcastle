<script lang="ts">
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	/** 지우기 전에 앱 확인 창으로 물어봄 */
	const ask = (message: string) => confirmAction({ title: "삭제", message, confirmLabel: "삭제", danger: true });
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
			name: "새 에셋",
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
	function addDropped(paths: string[]) {
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
		if (paths.length) toasts.success(`에셋 ${paths.length}개 추가함. 저장하면 게임에 들어감`);
	}
	$effect(() =>
		dropPaths({
			label: "게임 에셋으로 추가",
			hint: `그림·소리·글꼴. ${IMAGE_TYPES.includes(filter) ? `그림은 '${filter}' 종류로` : "그림은 기타 종류로"} 들어감`,
			accept: (f) => isImage(f) || isAudio(f) || isFont(f),
			onpaths: addDropped,
		}),
	);
	async function remove(type: string, index: number) {
		if (!await ask("이 에셋을 삭제할까요?")) return;
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
				{ asset: "기타", audio: "오디오", character: "캐릭터", cg: "일러스트", background: "배경" } as Record<
					string,
					string
				>
			)[type] || type
		);
	}
</script>

<section class="asset-manager">
	<header class="asset-toolbar">
		<div>
			<h3>전체 에셋</h3>
			<p>이미지, 오디오, 폰트 등 게임에서 사용하는 파일을 한곳에서 관리합니다.</p>
		</div>
		<div class="action-btn">
			<select aria-label="추가할 에셋 종류" bind:value={filter}>
				<option value="all">전체 보기</option>
				<option value="audio">오디오</option>
				<option value="character">캐릭터</option>
				<option value="cg">일러스트</option>
				<option value="background">배경</option>
				<option value="asset">기타</option>
			</select>
			<button class="add-btn" onclick={() => add(filter === "all" ? "asset" : filter)}>+ 에셋 추가</button>
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
							aria-label="이미지 선택"
							onclick={() => chooseFile(entry)}
						>
							{#if entry.asset.filePath}
								<img
									src={editorAssetPreviewUrl(assetRoot, entry.asset.filePath)}
									alt={entry.asset.name}
								/>
							{:else}
								<span>＋<small>이미지 선택</small> </span>
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
						aria-label={`${entry.asset.name} 미리 듣기`}
					>
					</audio>
				{/if}
				<div class="asset-fields">
					{#if entry.type !== "character"}
						<div class="field">
							<label>이름</label>
							<input bind:value={entry.asset.name} />
						</div>
					{/if}
					<div class="field">
						<label>키</label>
						<input bind:value={entry.asset.key} />
					</div>
					<Button variant="secondary" size="sm" onclick={() => chooseFile(entry)}>
						{entry.asset.filePath ? "파일 변경" : "파일 선택"}
					</Button>
					{#if entry.type === "asset"}
						<div class="field-row">
							<div class="field">
								<label>종류</label>
								<select bind:value={entry.asset.kind}>
									<option value="image">이미지</option>
									<option value="audio">오디오</option>
									<option value="font">폰트</option>
									<option value="other">기타</option>
								</select>
							</div>
							<div class="field">
								<label>분류</label>
								<input bind:value={entry.asset.category} />
							</div>
						</div>
					{:else if entry.type === "audio"}{@const audio = entry.asset as GameAudioAssetsInput}
						<div class="field-row">
							<div class="field">
								<label>용도</label>
								<select bind:value={audio.category}>
									<option value="voice">음성</option>
									<option value="sfx">효과음</option>
									<option value="bgm">배경 음악</option>
								</select>
							</div>
							<div class="field">
								<label>태그</label>
								<input bind:value={audio.tag} />
							</div>
						</div>
						{#if audio.category === "voice"}
							<div class="field">
								<label>캐릭터</label>
								<select bind:value={audio.gameCharacterId}>
									<option value={null}>선택 안 함</option>
									{#each characters as character}
										<option value={character.id}>
											{character.name}
										</option>
									{/each}
								</select>
							</div>
							<div class="field">
								<label>대사 기록</label>
								<textarea bind:value={audio.transcript} rows="2"></textarea>
							</div>
						{/if}
					{:else if entry.type === "character"}{@const characterAsset =
							entry.asset as GameCharacterAssetsInput}
						<div class="field-row">
							<div class="field">
								<label>캐릭터</label>
								<select bind:value={characterAsset.gameCharacterId}>
									<option value={null}>선택 안 함</option>
									{#each characters as character}
										<option value={character.id}>
											{character.name}
										</option>
									{/each}
								</select>
							</div>
							<div class="field">
								<label>표정</label>
								<input bind:value={characterAsset.emotion} />
							</div>
						</div>
						<div class="field">
							<label>사용처</label>
							<select bind:value={characterAsset.usage}>
								<option value="standing">스탠딩</option>
								<option value="map">맵</option>
							</select>
						</div>
					{:else if entry.type === "cg"}{@const cg = entry.asset as GameCgAssetsInput}
						<div class="field">
							<label>캐릭터</label>
							<select bind:value={cg.gameCharacterId}>
								<option value={null}>선택 안 함</option>
								{#each characters as character}
									<option value={character.id}>
										{character.name}
									</option>
								{/each}
							</select>
						</div>
						<div class="field">
							<label>설명</label>
							<textarea bind:value={cg.description} rows="2"></textarea>
						</div>
						<label class="check"> <input type="checkbox" bind:checked={cg.aiUsable} /> AI 사용 가능</label>
					{:else if entry.type === "background"}{@const background = entry.asset as GameBackgroundAssetsInput}
						<div class="field">
							<label>장소</label>
							<select bind:value={background.locationId}>
								<option value={null}>선택 안 함</option>
								{#each locations as location (location.id)}
									<option value={location.id}>
										{location.name}
									</option>
								{/each}
							</select>
						</div>
						<div class="field-row">
							<div class="field">
								<label>시간대</label>
								<select bind:value={background.timeId}>
									<option value={null}>모든 시간대</option>
									{#each timePeriods as period (period.id)}
										<option value={period.id}>
											{period.name}
										</option>
									{/each}
								</select>
							</div>
							<div class="field">
								<label>날씨</label>
								<select bind:value={background.weatherId}>
									<option value={null}>모든 날씨</option>
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
					<Button variant="danger" size="sm" onclick={() => remove(entry.type, entry.index)}>에셋 삭제</Button
					>
				</div>
			</article>
		{:else}
			<div class="empty">등록된 에셋이 없습니다. 위의 에셋 추가 버튼으로 시작하세요.</div>
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
