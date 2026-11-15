<script lang="ts">
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	/** 지우기 전에 앱 확인 창으로 물어봄 */
	const ask = (message: string) => confirmAction({ title: "삭제", message, confirmLabel: "삭제", danger: true });
	import type { GameAssetsInput, GameCharactersInput, GameCharacterAssetsInput } from "$features/game/types";
	import { editorAssetPreviewUrl } from "./assetPreview";
	import { open } from "@tauri-apps/plugin-dialog";
	import Input from "$/shared/ui/Input.svelte";
	import { Plus, Search, X } from "@lucide/svelte";
	import IconButton from "$/shared/ui/IconButton.svelte";

	type Props = {
		characters: GameCharactersInput[];
		gameId: string;
		assets: GameCharacterAssetsInput[];
		availableImages: GameAssetsInput[];
		assetRoot: string;
	};
	let {
		characters = $bindable([]),
		assets = $bindable([]),
		availableImages = [],
		gameId,
		assetRoot,
	}: Props = $props();

	let selectedIndex = $state<number | null>(characters.length > 0 ? 0 : null);
	let characterSearch = $state("");
	let filteredCharacters = $derived(
		characters
			.map((char, index) => ({ char, index }))
			.filter(({ char }) =>
				`${char.name} ${char.behaviorType ?? ""}`.toLowerCase().includes(characterSearch.trim().toLowerCase()),
			),
	);
	let activeChar = $derived(selectedIndex !== null && characters[selectedIndex] ? characters[selectedIndex] : null);
	let avatarPickerOpen = $state(false);
	let assetPickerIndex = $state<number | null>(null);

	function handleAddCharacter() {
		const newChar: GameCharactersInput = {
			id: crypto.randomUUID(),
			gameId: gameId,
			characterId: null, // 기본 캐릭터 중에서 게임 캐릭터로 추가하는 경우에만 사용
			name: "새로운 캐릭터",
			avatar: null,
			description: "",
			firstMessage: "",
			exampleDialogue: "",
			systemPromptOverride: "",
			behaviorType: "npc",
			worldInfoIds: [],
			tags: "",
			settingsJson: null,
		};
		characters.push(newChar);
		selectedIndex = characters.length - 1;
	}

	async function handleDeleteCharacter(index: number) {
		if (!(await ask(`"${characters[index].name}" 캐릭터를 삭제하시겠습니까?`))) return;
		const targetId = characters[index].id;
		assets = assets.filter((asset) => asset.gameCharacterId !== targetId);
		characters.splice(index, 1);
		selectedIndex = characters.length > 0 ? Math.max(0, index - 1) : null;
	}

	function handleAddCharacterAsset() {
		if (!activeChar?.id) return;
		assets.push({
			id: crypto.randomUUID(),
			gameId,
			key: `standing_${Date.now()}`,
			name: `${activeChar.name} 스탠딩`,
			category: "character",
			kind: "image",
			filePath: "",
			settingsJson: null,
			gameCharacterId: activeChar.id,
			usage: "standing",
			emotion: "neutral",
		});
	}

	function removeCharacterAsset(id: string | null) {
		assets = assets.filter((asset) => asset.id !== id);
	}

	function selectAvatar(filePath: string) {
		if (!activeChar) return;
		activeChar.avatar = filePath;
		avatarPickerOpen = false;
	}

	function selectCharacterImage(asset: GameCharacterAssetsInput, filePath: string) {
		asset.filePath = filePath;
		assetPickerIndex = null;
	}

	async function chooseImageFile() {
		const selected = await open({
			multiple: false,
			filters: [{ name: "이미지", extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp", "avif", "svg"] }],
		});
		return typeof selected === "string" ? selected : null;
	}

	async function chooseAvatarFile() {
		const path = await chooseImageFile();
		if (path && activeChar) activeChar.avatar = path;
		avatarPickerOpen = false;
	}

	async function chooseCharacterAssetFile(asset: GameCharacterAssetsInput) {
		const path = await chooseImageFile();
		if (path) asset.filePath = path;
		assetPickerIndex = null;
	}
</script>

<div class="sub-editor-layout">
	<!-- 좌측: 캐릭터 목록 -->
	<div class="item-list-pane">
		<div class="pane-header">
			<Input bind:value={characterSearch} placeholder="캐릭터 검색" aria-label="캐릭터 검색">
				{#snippet leading()}<Search />
				{/snippet}
			</Input>
			<IconButton label="캐릭터 추가" size="md" onclick={handleAddCharacter}><Plus /></IconButton>
		</div>

		<div class="scroll-list">
			{#each filteredCharacters as row (row.char.id || row.index)}
				{@const ev = row.char}{@const idx = row.index}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="list-item character-list-item"
					class:active={selectedIndex === idx}
					onclick={() => (selectedIndex = idx)}
				>
					<div class="character-item">
						<span class="character-avatar">
							{#if editorAssetPreviewUrl(assetRoot, ev.avatar)}<img
									src={editorAssetPreviewUrl(assetRoot, ev.avatar)}
									alt=""
								/>
							{:else}{ev.name.slice(0, 1) || "?"}{/if}</span
						>
						<div class="item-info">
							<span class="title"> {ev.name}</span>
							<span class="character-role"> {ev.behaviorType || "npc"}</span>
						</div>
					</div>
					<IconButton class="icon-del" size="sm" label="캐릭터 삭제" onclick={() => handleDeleteCharacter(idx)}><X /></IconButton>
				</div>
			{:else}
				<div class="empty-text">등록된 캐릭터가 없습니다.</div>
			{/each}
		</div>
	</div>

	<!-- 우측: 상세 편집 폼 -->
	<div class="item-detail-pane">
		{#if activeChar && selectedIndex !== null}
			<div class="detail-header">
				<h3>캐릭터 상세 편집</h3>
			</div>
			<div class="form-grid">
				<details class="accordion" open>
					<summary><span>기본 정보</span><small>이름, 대표 이미지, 캐릭터 설명</small></summary>
					<div class="accordion-content">
						<div class="character-profile">
							<div class="field avatar-field">
								<label>아바타</label>
								<button
									class="avatar-image-button"
									type="button"
									aria-label="아바타 이미지 선택"
									onclick={() => (avatarPickerOpen = !avatarPickerOpen)}
								>
									{#if editorAssetPreviewUrl(assetRoot, activeChar.avatar)}<img
											src={editorAssetPreviewUrl(assetRoot, activeChar.avatar)}
											alt="선택된 아바타"
										/>
									{:else}<span>＋<small>이미지 선택</small></span>
									{/if}
								</button>
								{#if activeChar.avatar}<button
										class="clear-avatar"
										type="button"
										onclick={() => (activeChar.avatar = null)}>아바타 지우기</button
									>
								{/if}
								{#if avatarPickerOpen}<div class="image-picker">
										<div class="picker-heading">
											<strong>이미지 선택</strong>
											<div>
												<button type="button" onclick={chooseAvatarFile}>파일 선택</button
												><button type="button" onclick={() => (avatarPickerOpen = false)}
													>닫기</button
												>
											</div>
										</div>
										<div class="picker-grid">
											{#each availableImages as imageAsset (`${imageAsset.id}-${imageAsset.key}`)}<button
													type="button"
													class="picker-image"
													onclick={() => selectAvatar(imageAsset.filePath)}
													><span>
														{#if editorAssetPreviewUrl(assetRoot, imageAsset.filePath)}<img
																src={editorAssetPreviewUrl(
																	assetRoot,
																	imageAsset.filePath,
																)}
																alt=""
															/>
														{:else}▧{/if}</span
													><small> {imageAsset.key}</small></button
												>
											{:else}<p class="asset-empty">
													등록된 이미지가 없습니다. 파일 선택으로 추가하세요.
												</p>
											{/each}
										</div>
									</div>
								{/if}
							</div>
							<div class="character-profile-fields">
								<div class="field">
									<label for="char_name">캐릭터 이름</label>
									<input id="char_name" type="text" bind:value={activeChar.name} />
								</div>
								<div class="field">
									<label for="char_des">캐릭터 정보</label>
									<textarea
										id="char_des"
										bind:value={activeChar.description}
										rows="5"
										placeholder="캐릭터의 배경 설정 및 성격"></textarea>
								</div>
							</div>
						</div>
					</div>
				</details>
				<details class="accordion" open>
					<summary><span>대화 설정</span><small>첫 대사와 캐릭터 응답 가이드</small></summary>
					<div class="accordion-content">
						<div class="field">
							<label for="char_first">첫 대사 (First Message)</label>
							<textarea
								id="char_first"
								bind:value={activeChar.firstMessage}
								rows="2"
								placeholder="채팅 시작 시 건네는 첫 인사"></textarea>
						</div>
						<div class="field">
							<label for="char_behavior">행동 타입 (Behavior Type)</label>
							<input
								id="char_behavior"
								type="text"
								bind:value={activeChar.behaviorType}
								placeholder="예: npc, companion 등"
							/>
						</div>
						<div class="field">
							<label for="char_example">예시 대화 (Example Dialogue)</label>
							<textarea
								id="char_example"
								bind:value={activeChar.exampleDialogue}
								rows="3"
								placeholder="<START>\n"></textarea>
						</div>
					</div>
				</details>
				<details class="accordion character-assets" open>
					<summary><span>캐릭터 이미지</span><small>스탠딩과 맵 이미지 관리</small></summary>
					<div class="accordion-content">
						<div class="section-heading">
							<button class="add-btn" onclick={handleAddCharacterAsset} disabled={!activeChar.id}
								>+ 이미지 추가</button
							>
						</div>
						{#if assets.filter((asset) => asset.gameCharacterId === activeChar.id).length}
							<div class="character-asset-list">
								{#each assets.filter((asset) => asset.gameCharacterId === activeChar.id) as asset, assetIndex (asset.id)}
									<article class="character-asset-card">
										<button
											class="asset-preview"
											type="button"
											aria-label="이미지 파일 선택"
											onclick={() =>
												(assetPickerIndex =
													assetPickerIndex === assetIndex ? null : assetIndex)}
										>
											{#if editorAssetPreviewUrl(assetRoot, asset.filePath)}<img
													src={editorAssetPreviewUrl(assetRoot, asset.filePath)}
													alt={asset.name}
												/>
											{:else}<span>▧</span>
											{/if}
										</button>
										{#if assetPickerIndex === assetIndex}<div
												class="image-picker asset-image-picker"
											>
												<div class="picker-heading">
													<strong>이미지 선택</strong>
													<div>
														<button
															type="button"
															onclick={() => chooseCharacterAssetFile(asset)}
															>파일 선택</button
														><button type="button" onclick={() => (assetPickerIndex = null)}
															>닫기</button
														>
													</div>
												</div>
												<div class="picker-grid">
													{#each availableImages as imageAsset (`${imageAsset.id}-${imageAsset.key}`)}<button
															type="button"
															class="picker-image"
															onclick={() =>
																selectCharacterImage(asset, imageAsset.filePath)}
															><span>
																{#if editorAssetPreviewUrl(assetRoot, imageAsset.filePath)}<img
																		src={editorAssetPreviewUrl(
																			assetRoot,
																			imageAsset.filePath,
																		)}
																		alt=""
																	/>
																{:else}▧{/if}</span
															><small> {imageAsset.key}</small></button
														>
													{:else}<p class="asset-empty">
															등록된 이미지가 없습니다. 파일 선택으로 추가하세요.
														</p>
													{/each}
												</div>
											</div>
										{/if}
										<div class="asset-fields">
											<label>에셋 키<input bind:value={asset.key} /></label>
											<div class="asset-field-row">
												<label
													>유형<select bind:value={asset.usage}
														><option value="standing">스탠딩</option><option value="map"
															>맵 이미지</option
														></select
													></label
												><label
													>표정<input
														bind:value={asset.emotion}
														placeholder="neutral"
														disabled={asset.usage === "map"}
													/></label
												>
											</div>
										</div>
										<button
											class="asset-remove"
											aria-label="이미지 삭제"
											onclick={() => removeCharacterAsset(asset.id)}>×</button
										>
									</article>
								{/each}
							</div>
						{:else}<div class="asset-empty">
								아직 등록된 이미지가 없습니다. 이곳에서 캐릭터 이미지를 추가하세요.
							</div>
						{/if}
					</div>
				</details>
			</div>
		{:else}
			<div class="no-selection">편집할 캐릭터를 선택하거나 새로 추가해주세요.</div>
		{/if}
	</div>
</div>

<style>
	.sub-editor-layout {
		display: grid;
		grid-template-columns: 320px 1fr;
		height: 100%;
		min-height: 500px;
	}
	.item-list-pane,
	.item-detail-pane {
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.add-btn {
		background: var(--accent);
		color: var(--text-on-accent);
		border: none;
		padding: 0.4rem 0.8rem;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}
	.add-btn:hover {
		background: var(--accent-hover);
	}
	.scroll-list {
		flex: 1;
		overflow-y: auto;
		padding: 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.list-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1rem;
		background: var(--surface);
		border-radius: 6px;
		cursor: pointer;
		border: 1px solid transparent;
		transition: all 0.15s;
	}
	.list-item:hover {
		border-color: var(--border-strong);
	}
	.list-item.active {
		border-color: var(--accent);
		background: var(--accent-soft);
	}

	.empty-text {
		text-align: center;
		color: var(--text-faint);
		padding: 2rem;
		font-size: 0.85rem;
	}
	.item-detail-pane {
		padding: 1.5rem;
		overflow-y: auto;
	}
	.detail-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.2rem;
		border-bottom: 1px solid var(--border);
		padding-bottom: 0.8rem;
	}
	.detail-header h3 {
		margin: 0;
		font-size: 1.1rem;
		color: var(--text);
	}
	.form-grid {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.field label {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-muted);
	}
	.field input,
	.field textarea {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.6rem;
		color: var(--text);
		font-size: 0.95rem;
	}
	.field input:focus,
	.field textarea:focus {
		outline: none;
		border-color: var(--accent);
	}
	.no-selection {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--text-faint);
		font-size: 0.95rem;
		text-align: center;
	}
	.character-item {
		display: flex;
		justify-content: flex-start;
		gap: 0.7rem;
	}

	.character-avatar {
		display: grid;
		place-items: center;
		flex: none;
		width: 2.55rem;
		height: 2.55rem;
		overflow: hidden;
		border-radius: 50%;
		background: var(--surface-active);
		color: var(--accent);
		font-size: 1rem;
		font-weight: 700;
	}
	.character-avatar img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.character-role {
		color: var(--text-muted);
		font-size: 0.72rem;
		text-transform: capitalize;
	}
	.character-profile {
		display: grid;
		grid-template-columns: minmax(150px, 190px) minmax(0, 1fr);
		gap: 1.25rem;
		align-items: start;
	}
	.character-profile-fields {
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
		min-width: 0;
	}
	.avatar-field {
		position: relative;
		align-items: flex-start;
	}
	.avatar-image-button {
		display: grid;
		place-items: center;
		width: 160px;
		height: 160px;
		overflow: hidden;
		padding: 0;
		border: 1px dashed var(--border-strong);
		border-radius: var(--radius-md);
		background: var(--surface-2);
		color: var(--text-muted);
		cursor: pointer;
	}
	.avatar-image-button img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.avatar-image-button span {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.3rem;
		color: var(--accent);
		font-size: 1.7rem;
	}
	.avatar-image-button small {
		color: var(--text-muted);
		font-size: 0.75rem;
	}
	.clear-avatar {
		border: 0;
		padding: 0.35rem 0;
		background: none;
		color: var(--text-muted);
		font: inherit;
		font-size: 0.76rem;
		cursor: pointer;
	}
	.image-picker {
		margin-top: 0.65rem;
		padding: 0.75rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-2);
	}
	.avatar-field > .image-picker {
		position: absolute;
		z-index: 4;
		top: 1.35rem;
		left: calc(100% + 0.75rem);
		width: min(560px, calc(100vw - 520px));
		margin: 0;
		box-shadow: var(--shadow-md);
	}
	.picker-heading {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.65rem;
		color: var(--text);
		font-size: 0.82rem;
	}
	.picker-heading button {
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		padding: 0.25rem 0.5rem;
		background: var(--surface);
		color: var(--text-muted);
		cursor: pointer;
	}
	.picker-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
		gap: 0.5rem;
		max-height: 260px;
		overflow: auto;
	}
	.picker-image {
		min-width: 0;
		padding: 0.35rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		background: var(--surface);
		color: var(--text);
		cursor: pointer;
	}
	.picker-image:hover {
		border-color: var(--accent);
	}
	.picker-image span {
		display: grid;
		place-items: center;
		height: 75px;
		overflow: hidden;
		background: var(--surface-2);
		color: var(--text-muted);
	}
	.picker-image img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.picker-image small {
		display: block;
		overflow: hidden;
		padding-top: 0.3rem;
		color: var(--text-muted);
		font-size: 0.68rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.asset-image-picker {
		grid-column: 1/-1;
	}
	.accordion {
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-sm);
		overflow: hidden;
	}
	.accordion summary {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		padding: 0.9rem 1rem;
		color: var(--text);
		font-size: 0.9rem;
		font-weight: 700;
		cursor: pointer;
		list-style: none;
	}
	.accordion summary::-webkit-details-marker {
		display: none;
	}
	.accordion summary::after {
		content: "›";
		margin-left: auto;
		color: var(--text-muted);
		font-size: 1.1rem;
		transition: transform 0.15s;
	}
	.accordion[open] summary::after {
		transform: rotate(90deg);
	}
	.accordion summary small {
		margin-left: 0;
		color: var(--text-muted);
		font-size: 0.72rem;
		font-weight: 400;
	}
	.accordion-content {
		padding: 0 1rem 1rem;
	}
	.character-assets {
		margin-top: 0.5rem;
	}
	.section-heading {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.8rem;
		margin-bottom: 0.8rem;
	}
	.section-heading h4 {
		margin: 0;
		color: var(--text);
		font-size: 0.95rem;
	}
	.section-heading p {
		margin: 0.25rem 0 0;
		color: var(--text-muted);
		font-size: 0.78rem;
	}
	.asset-hint {
		color: var(--text-muted);
		font-size: 0.76rem;
	}
	.character-asset-list {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(100%, 260px), 1fr));
		gap: 0.75rem;
	}
	.character-asset-card {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: 0.7rem;
		padding: 0.7rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-2);
	}
	.asset-preview {
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
	.asset-fields {
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
		min-width: 0;
	}
	.asset-fields label {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		color: var(--text-muted);
		font-size: 0.72rem;
		font-weight: 600;
	}
	.asset-fields input,
	.asset-fields select {
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
		padding: 0.48rem 0.6rem;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.8rem;
	}
	.asset-field-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.55rem;
	}
	.asset-remove {
		position: absolute;
		top: 1rem;
		right: 1rem;
		border: 0;
		border-radius: 50%;
		width: 1.8rem;
		height: 1.8rem;
		background: var(--surface-2);
		color: var(--text-muted);
		font-size: 1.2rem;
		cursor: pointer;
	}
	.asset-remove:hover {
		color: var(--danger);
	}
	.asset-empty {
		padding: 1rem;
		border: 1px dashed var(--border-strong);
		border-radius: var(--radius-md);
		color: var(--text-muted);
		font-size: 0.8rem;
		text-align: center;
	}
	@media (max-width: 640px) {
		.character-profile {
			grid-template-columns: 1fr;
		}
		.avatar-field {
			align-items: stretch;
		}
		.avatar-image-button {
			align-self: flex-start;
		}
		.avatar-field > .image-picker {
			position: static;
			width: auto;
			margin-top: 0.65rem;
			box-shadow: none;
		}
		.asset-field-row {
			grid-template-columns: 1fr;
		}
		.accordion summary {
			flex-wrap: wrap;
		}
		.accordion summary small {
			width: 100%;
			margin-left: 1.75rem;
		}
	}
</style>
