<script lang="ts">
	import { onMount } from "svelte";
	import type { PackageData } from "$/lib/type/game";
	import SceneEditorSection from "./editor/SceneEditorSection.svelte";
	import StateDefEditorSection from "./editor/StateDefEditorSection.svelte";
	import { gameManagerCommands } from "$/ipc/commands";
	import MapEditorSection from "./editor/MapEditorSection.svelte";
	import CharacterEditorSection from "./editor/CharacterEditorSection.svelte";
	import WeatherEditorSection from "./editor/WeatherEditorSection.svelte";
	import TimePeriodEditorSection from "./editor/TimePeriodEditorSection.svelte";
	import EventEditorSection from "./editor/EventEditorSection.svelte";
	import AssetEditorSection from "./editor/AssetEditorSection.svelte";
	import { handleGoBack } from "$/app/routes";
	import { appDataDir, join } from "@tauri-apps/api/path";
	import Icon from "$/components/ui/Icon.svelte";
	import Header from "$/components/Header.svelte";

	type Props = { params: { gameId: string } };
	// props로 라우터 파라미터 전달받음 (svelte-spa-router 규격)
	let { params }: Props = $props();

	let packageData = $state<PackageData | null>(null);
	let isLoading = $state(true);
	let isSaving = $state(false);
	let assetRoot = $state("");
	let activeTab = $state<
		"project" | "scenes" | "stateDefs" | "characters" | "maps" | "weather" | "time" | "event" | "assets"
	>("project");

	async function loadPackageData() {
		try {
			isLoading = true;
			try {
				packageData = await gameManagerCommands.getPackageData({ gameId: params.gameId });
			} catch (reloadError) {
				console.warn("Saved package, but could not refresh normalized asset paths:", reloadError);
			}
		} catch (err) {
			console.error("Failed to load package data:", err, params.gameId);
			alert("패키지 데이터를 불러오지 못했습니다.");
		} finally {
			isLoading = false;
		}
	}

	async function loadAssetRoot() {
		try {
			assetRoot = await join(await appDataDir(), "games", params.gameId);
		} catch (err) {
			console.warn("Game asset previews are unavailable:", err);
		}
	}

	async function handleSave() {
		if (!packageData) return;
		try {
			isSaving = true;
			const report = await gameManagerCommands.savePackage({
				gameId: params.gameId,
				data: packageData,
			});
			packageData = await gameManagerCommands.getPackageData({ gameId: params.gameId });
			console.log("Save report:", report);
			alert("저장되었습니다!");
		} catch (err) {
			console.error("Failed to save package:", err);
			alert("저장 중 오류가 발생했습니다.");
		} finally {
			isSaving = false;
		}
	}

	onMount(() => {
		loadPackageData();
		loadAssetRoot();
	});
</script>

<div class="editor-container game-editor">
	{#if isLoading}
		<div class="loading-box">에디터 데이터 불러오는 중...</div>
	{:else if !packageData}
		<div class="error-box">데이터가 존재하지 않습니다.</div>
	{:else}
		<!-- 에디터 상단 툴바 -->
		<Header class="editor-header">
			<div class="left-group">
				<button class="back-btn" onclick={handleGoBack}><Icon name={"ChevronLeft"} /></button>
				<div class="project-heading"><h2>{packageData.project.title}</h2></div>
			</div>
			<div class="right-group">
				<button class="save-btn" onclick={handleSave} disabled={isSaving}>
					{isSaving ? "저장 중..." : "변경사항 저장"}
				</button>
			</div>
		</Header>

		<div class="editor-workspace">
			<!-- 탭 네비게이션 -->
			<nav class="editor-tabs" aria-label="에디터 탭">
				<button class:active={activeTab === "project"} onclick={() => (activeTab = "project")}>
					프로젝트
				</button>
				<button class:active={activeTab === "scenes"} onclick={() => (activeTab = "scenes")}>
					씬 목록 ({packageData.scenes.length})
				</button>
				<button class:active={activeTab === "stateDefs"} onclick={() => (activeTab = "stateDefs")}>
					변수({packageData.stateDefs.length})
				</button>
				<button class:active={activeTab === "characters"} onclick={() => (activeTab = "characters")}>
					캐릭터 ({packageData.characters.length})
				</button>
				<button class:active={activeTab === "maps"} onclick={() => (activeTab = "maps")}>
					맵 · 지역 · 이동 ({packageData.maps.length} · {packageData.locations.length} · {packageData
						.locationConnections.length})
				</button>
				<button class:active={activeTab === "weather"} onclick={() => (activeTab = "weather")}>
					날씨 ({packageData.weatherTypes.length})
				</button>
				<button class:active={activeTab === "time"} onclick={() => (activeTab = "time")}>
					시간 ({packageData.timePeriods.length})
				</button>
				<button class:active={activeTab === "event"} onclick={() => (activeTab = "event")}>
					이벤트 ({packageData.events.length})
				</button>
				<button class:active={activeTab === "assets"} onclick={() => (activeTab = "assets")}>
					에셋 ({packageData.assets.length +
						packageData.audioAssets.length +
						packageData.characterAssets.length +
						packageData.cgAssets.length +
						packageData.backgroundAssets.length})
				</button>
			</nav>

			<!-- 탭 콘텐츠 바디 -->
			<main class="editor-body">
				{#if activeTab === "project"}
					<div class="form-section">
						<details class="editor-accordion" open>
							<summary>기본 설정</summary>
							<div class="editor-accordion-content">
								<div class="field">
									<label for="project_title">프로젝트 제목</label>
									<input id="project_title" type="text" bind:value={packageData.project.title} />
								</div>
								<div class="field">
									<label for="project_des">프로젝트 설명</label>
									<textarea id="project_des" bind:value={packageData.project.description} rows="3"
									></textarea>
								</div>
								<div class="field">
									<label for="engine_api">엔진 API 버전</label>
									<input
										id="engine_api"
										type="text"
										bind:value={packageData.project.apiVersion}
										disabled
									/>
								</div>
								<div class="field">
									<label for="custom_css">커스텀 CSS</label>
									<textarea
										id="custom_css"
										class="code-font"
										bind:value={packageData.project.customCss}
										rows="5"></textarea>
								</div>
							</div>
						</details>
					</div>
				{:else if activeTab === "scenes"}
					<SceneEditorSection bind:scenes={packageData.scenes} gameId={packageData.project.id} />
				{:else if activeTab === "stateDefs"}
					<StateDefEditorSection bind:stateDefs={packageData.stateDefs} gameId={packageData.project.id} />
				{:else if activeTab === "characters"}
					<CharacterEditorSection
						bind:characters={packageData.characters}
						bind:assets={packageData.characterAssets}
						availableImages={[
							...packageData.assets,
							...packageData.characterAssets,
							...packageData.cgAssets,
							...packageData.backgroundAssets,
						].filter((asset) => asset.kind === "image" && !!asset.filePath)}
						gameId={packageData.project.id}
						{assetRoot}
					/>
				{:else if activeTab === "maps"}
					<MapEditorSection
						bind:zones={packageData.zones}
						bind:maps={packageData.maps}
						bind:locations={packageData.locations}
						bind:backgroundAssets={packageData.backgroundAssets}
						bind:connections={packageData.locationConnections}
						bind:interactables={packageData.interactables}
						events={packageData.events}
						timePeriods={packageData.timePeriods}
						weatherTypes={packageData.weatherTypes}
						gameId={packageData.project.id}
						{assetRoot}
					/>
				{:else if activeTab === "weather"}
					<WeatherEditorSection
						bind:weatherTypes={packageData.weatherTypes}
						bind:weatherTransitions={packageData.weatherTransitions}
						maps={packageData.maps}
						gameId={packageData.project.id}
					/>
				{:else if activeTab === "time"}
					<TimePeriodEditorSection
						bind:timePeriods={packageData.timePeriods}
						gameId={packageData.project.id}
					/>
				{:else if activeTab === "event"}
					<EventEditorSection
						bind:events={packageData.events}
						items={packageData.items}
						stateDefs={packageData.stateDefs}
						scenes={packageData.scenes}
						characters={packageData.characters}
						gameId={packageData.project.id}
					/>
				{:else if activeTab === "assets"}
					<AssetEditorSection
						bind:assets={packageData.assets}
						bind:audioAssets={packageData.audioAssets}
						bind:characterAssets={packageData.characterAssets}
						bind:cgAssets={packageData.cgAssets}
						bind:backgroundAssets={packageData.backgroundAssets}
						characters={packageData.characters}
						locations={packageData.locations}
						timePeriods={packageData.timePeriods}
						weatherTypes={packageData.weatherTypes}
						gameId={packageData.project.id}
						{assetRoot}
					/>
				{/if}
			</main>
		</div>
	{/if}
</div>

<style>
	.editor-container {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		overflow: hidden;
		background: var(--background-primary);
		color: var(--text-color);
		font-family: inherit;
	}
	.loading-box,
	.error-box {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 70vh;
		color: var(--text-muted);
		font-size: 1.1rem;
	}
	.editor-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		padding: 1rem clamp(1rem, 4vw, 3.5rem);
		background: var(--background-primary);
		border-bottom: 1px solid var(--border-color);
		flex: 0 0 auto;
	}
	.left-group {
		display: flex;
		align-items: center;
		gap: 1.25rem;
		min-width: 0;
	}
	.project-heading {
		display: flex;
		align-items: baseline;
		gap: 0.65rem;
		flex-wrap: wrap;
		min-width: 0;
	}
	.project-heading h2 {
		margin: 0;
		font-size: 1.12rem;
		font-weight: 700;
	}
	.left-group h2 {
		font-size: 1.12rem;
		margin: 0;
	}
	.back-btn,
	.save-btn,
	.primary-action {
		border: 1px solid var(--border-color);
		border-radius: var(--radius-md);
		padding: 0.62rem 0.95rem;
		font: inherit;
		font-size: 0.84rem;
		font-weight: 650;
		cursor: pointer;
		transition:
			background 0.15s,
			border-color 0.15s,
			transform 0.15s;
	}
	.back-btn {
		background: var(--background-secondary);
		color: var(--text-muted);
		white-space: nowrap;
	}
	.back-btn:hover {
		background: var(--button-hover-bg);
		color: var(--text-color);
	}
	.save-btn {
		background: var(--accent-color);
		color: white;
		border-color: transparent;
		box-shadow: var(--shadow-sm);
		white-space: nowrap;
	}
	.save-btn:hover {
		background: var(--accent-color-hover);
		transform: translateY(-1px);
	}
	.save-btn:disabled {
		opacity: 0.65;
		cursor: wait;
		transform: none;
	}
	.editor-tabs {
		display: flex;
		flex-direction: column;
		flex: 0 0 230px;
		background: var(--background-secondary);
		border-right: 1px solid var(--border-color);
		padding: 0.85rem 0.65rem;
		gap: 0.25rem;
		overflow-y: auto;
		overflow-x: hidden;
		scrollbar-width: thin;
		min-height: 0;
	}
	.editor-tabs button {
		position: relative;
		flex: none;
		width: 100%;
		text-align: left;
		background: transparent;
		border: 0;
		border-left: 2px solid transparent;
		border-radius: var(--radius-md);
		color: var(--text-muted);
		padding: 0.72rem 0.75rem;
		font: inherit;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
		white-space: normal;
		transition:
			color 0.15s,
			background 0.15s;
	}
	.editor-tabs button:hover {
		color: var(--text-color);
		background: var(--button-hover-bg);
	}
	.editor-tabs button.active {
		color: var(--accent-color);
		border-left-color: var(--accent-color);
		background: color-mix(in srgb, var(--accent-color) 9%, var(--background-secondary));
	}
	.editor-workspace {
		display: flex;
		flex: 1;
		min-height: 0;
		width: 100%;
	}
	.editor-body {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		min-width: 0;
		width: 100%;
		max-width: none;
		margin: 0;
		box-sizing: border-box;
	}
	.form-section {
		max-width: none;
		margin: 0;
		background: var(--card-bg);
		padding: clamp(1.15rem, 3vw, 2rem);
		border-radius: var(--radius-lg);
		border: 1px solid var(--border-color);
		box-shadow: var(--shadow-sm);
		display: flex;
		flex-direction: column;
		gap: 1.2rem;
	}
	:global(.game-editor .editor-accordion) {
		border: 1px solid var(--border-color);
		border-radius: var(--radius-md);
		background: var(--card-bg);
		overflow: hidden;
	}
	:global(.game-editor .editor-accordion > summary) {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		list-style: none;
		cursor: pointer;
		padding: 0.85rem 1rem;
		color: var(--text-color);
		font-weight: 650;
	}
	:global(.game-editor .editor-accordion > summary::-webkit-details-marker) {
		display: none;
	}
	:global(.game-editor .editor-accordion > summary::after) {
		content: "›";
		margin-left: auto;
		color: var(--text-muted);
		font-size: 1.2rem;
		line-height: 1;
		transform: rotate(0);
		transition: transform 0.15s;
	}
	:global(.game-editor .editor-accordion[open] > summary::after) {
		transform: rotate(90deg);
	}
	:global(.game-editor .editor-accordion-content) {
		padding: 0 1rem 1rem;
	}
	:global(.game-editor .form-grid > .editor-accordion) {
		grid-column: 1 / -1;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
	}
	.field label {
		font-size: 0.82rem;
		font-weight: 650;
		color: var(--text-muted);
	}
	.field input,
	.field textarea,
	.field select {
		background: var(--input-bg);
		border: 1px solid var(--input-border-color);
		border-radius: var(--radius-md);
		padding: 0.68rem 0.78rem;
		color: var(--input-color);
		font: inherit;
		font-size: 0.9rem;
	}
	.field input:focus,
	.field textarea:focus,
	.field select:focus {
		outline: none;
		border-color: var(--accent-color);
		box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-color) 16%, transparent);
	}
	.field input:disabled {
		opacity: 0.65;
	}
	.code-font {
		font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
		font-size: 0.86rem;
	}

	:global(.game-editor .sub-editor-layout) {
		grid-template-columns: minmax(230px, 290px) minmax(0, 1fr);
		min-height: 520px;
	}

	:global(.game-editor .pane-header) {
		padding: 0.9rem 1rem;
		background: var(--background-secondary);
		border-bottom: 1px solid var(--border-color);
	}
	:global(.game-editor .list-search) {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0.7rem 0.65rem 0.25rem;
		padding: 0.55rem 0.7rem;
		border: 1px solid var(--border-color);
		border-radius: var(--radius-md);
		background: var(--input-bg);
		color: var(--text-muted);
	}
	:global(.game-editor .list-search input) {
		flex: 1;
		min-width: 0;
		border: 0;
		outline: 0;
		background: transparent;
		color: var(--input-color);
		font: inherit;
		font-size: 0.8rem;
	}
	:global(.game-editor .pane-header h4),
	:global(.game-editor .detail-header h3) {
		color: var(--text-color);
	}
	:global(.game-editor .add-btn),
	:global(.game-editor .primary-action) {
		padding: 0.48rem 0.7rem;
		background: var(--background-secondary);
		color: var(--text-color);
		border: 1px solid var(--border-color);
		border-radius: var(--radius-md);
		font: inherit;
		font-size: 0.76rem;
		font-weight: 600;
		cursor: pointer;
		box-shadow: none;
	}
	:global(.game-editor .add-btn:hover),
	:global(.game-editor .primary-action:hover) {
		background: var(--button-hover-bg);
		border-color: var(--border-strong-color);
		transform: none;
	}
	:global(.game-editor .add-btn:disabled) {
		background: var(--background-secondary);
		color: var(--text-muted);
		border-color: var(--border-color);
		opacity: 0.6;
		cursor: not-allowed;
	}
	:global(.game-editor .scroll-list) {
		padding: 0.55rem;
		gap: 0.3rem;
	}
	:global(.game-editor .list-item) {
		padding: 0.7rem 0.75rem;
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--radius-md);
	}
	:global(.game-editor .list-item:hover) {
		border-color: var(--border-color);
		background: var(--button-hover-bg);
	}
	:global(.game-editor .list-item.active) {
		border-color: color-mix(in srgb, var(--accent-color) 35%, var(--border-color));
		background: color-mix(in srgb, var(--accent-color) 9%, var(--card-bg));
	}
	:global(.game-editor .item-info .title),
	:global(.game-editor .detail-header h3) {
		color: var(--text-color);
	}
	:global(.game-editor .item-info .key),
	:global(.game-editor .item-info .type),
	:global(.game-editor .empty-text),
	:global(.game-editor .no-selection) {
		color: var(--text-muted);
	}
	:global(.game-editor .item-info .title code) {
		color: var(--accent-color);
		background: var(--background-secondary);
	}
	:global(.game-editor .badge.ending) {
		background: var(--green-3);
		color: var(--green-11);
	}
	:global(.game-editor .item-detail-pane) {
		padding: clamp(1rem, 2vw, 1.5rem);
	}
	:global(.game-editor .detail-header) {
		border-color: var(--border-color);
	}
	:global(.game-editor .delete-btn) {
		background: color-mix(in srgb, var(--color-red) 12%, transparent);
		color: var(--color-red);
		border: 1px solid color-mix(in srgb, var(--color-red) 24%, transparent);
		border-radius: var(--radius-md);
		padding: 0.42rem 0.7rem;
		font: inherit;
		font-size: 0.78rem;
		font-weight: 650;
		cursor: pointer;
	}
	:global(.game-editor .field input),
	:global(.game-editor .field select),
	:global(.game-editor .field textarea) {
		background: var(--input-bg);
		color: var(--input-color);
		border-color: var(--input-border-color);
		border-radius: var(--radius-md);
	}
	:global(.game-editor .field label),
	:global(.game-editor .checkbox-label) {
		color: var(--text-muted);
	}
	:global(.game-editor .field input:focus),
	:global(.game-editor .field select:focus),
	:global(.game-editor .field textarea:focus) {
		border-color: var(--accent-color);
	}
	@media (max-width: 1100px) {
		:global(.game-editor .map-hierarchy-layout) {
			grid-template-columns: minmax(230px, 290px) minmax(0, 1fr);
		}
	}
	@media (max-width: 760px) {
		.editor-header {
			align-items: flex-start;
		}
		.left-group {
			gap: 0.65rem;
		}
		.project-heading {
			gap: 0.35rem;
		}
		.project-heading .eyebrow {
			width: 100%;
		}
		.project-heading h2 {
			font-size: 1rem;
		}
		.save-btn {
			padding: 0.55rem 0.7rem;
			font-size: 0.76rem;
		}
		.editor-tabs {
			flex-basis: 170px;
			padding: 0.65rem 0.45rem;
		}
		.editor-tabs button {
			padding: 0.65rem 0.55rem;
			font-size: 0.75rem;
		}
		:global(.game-editor .sub-editor-layout) {
			grid-template-columns: 1fr;
			min-height: 0;
		}
		:global(.game-editor .item-list-pane) {
			min-height: 230px;
			max-height: 310px;
		}
		:global(.game-editor .map-hierarchy-layout) {
			grid-template-columns: 1fr;
		}
		.record-heading {
			align-items: flex-start;
			flex-direction: column;
		}
	}
</style>
