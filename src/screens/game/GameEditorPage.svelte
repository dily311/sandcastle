<script lang="ts">
	import { onMount } from "svelte";
	import type { PackageData } from "$/lib/type/game";
	import SceneEditorSection from "./editor/SceneEditorSection.svelte";
	import StateDefEditorSection from "./editor/StateDefEditorSection.svelte";
	import { gameManagerCommands } from "$/ipc/commands";
	import MapEditorSection from "./editor/MapEditorSection.svelte";
	import CharacterEditorSection from "./editor/CharacterEditorSection.svelte";
	import LocationConnectionEditorSection from "./editor/LocationConnectionEditorSection.svelte";
	import WeatherEditorSection from "./editor/WeatherEditorSection.svelte";
	import TimePeriodEditorSection from "./editor/TimePeriodEditorSection.svelte";
	import EventEditorSection from "./editor/EventEditorSection.svelte";
	import { handleGoBack } from "$/app/routes";

	type Props = { params: { gameId: string } };
	// props로 라우터 파라미터 전달받음 (svelte-spa-router 규격)
	let { params }: Props = $props();

	let packageData = $state<PackageData | null>(null);
	let isLoading = $state(true);
	let isSaving = $state(false);
	let activeTab = $state<"project" | "scenes" | "stateDefs" | "characters" | "maps" | "connections" | "weather" | "time" | "event">(
		"project",
	);

	async function loadPackageData() {
		try {
			isLoading = true;
			packageData = await gameManagerCommands.getPackageData({ gameId: params.gameId });
		} catch (err) {
			console.error("Failed to load package data:", err, params.gameId);
			alert("패키지 데이터를 불러오지 못했습니다.");
		} finally {
			isLoading = false;
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
	});
</script>

<div class="editor-container">
	{#if isLoading}
		<div class="loading-box">에디터 데이터 불러오는 중...</div>
	{:else if !packageData}
		<div class="error-box">데이터가 존재하지 않습니다.</div>
	{:else}
		<!-- 에디터 상단 툴바 -->
		<header class="editor-header">
			<div class="left-group">
				<button class="back-btn" onclick={handleGoBack}>← 목록으로</button>
				<h2>{packageData.project.title} <span class="sub-id">({packageData.project.id})</span></h2>
			</div>
			<div class="right-group">
				<button class="save-btn" onclick={handleSave} disabled={isSaving}>
					{isSaving ? "저장 중..." : "저장하기"}
				</button>
			</div>
		</header>

		<!-- 탭 네비게이션 -->
		<nav class="editor-tabs">
			<button class:active={activeTab === "project"} onclick={() => (activeTab = "project")}>
				프로젝트 설정
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
				맵 / 지역 ({packageData.maps.length})
			</button>
			<button class:active={activeTab === "connections"} onclick={() => (activeTab = "connections")}>
				이동 경로 ({packageData.locationConnections.length})
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
		</nav>

		<!-- 탭 콘텐츠 바디 -->
		<main class="editor-body">
			{#if activeTab === "project"}
				<div class="form-section">
					<h3>기본 설정</h3>
					<div class="field">
						<label for="project_title">프로젝트 제목</label>
						<input id="project_title" type="text" bind:value={packageData.project.title} />
					</div>
					<div class="field">
						<label for="project_des">프로젝트 설명</label>
						<textarea id="project_des" bind:value={packageData.project.description} rows="3"></textarea>
					</div>
					<div class="field">
						<label for="engine_api">엔진 API 버전</label>
						<input id="engine_api" type="text" bind:value={packageData.project.apiVersion} disabled />
					</div>
					<div class="field">
						<label for="custom_css">커스텀 CSS</label>
						<textarea id="custom_css" class="code-font" bind:value={packageData.project.customCss} rows="5"></textarea>
					</div>
				</div>
			{:else if activeTab === "scenes"}
				<SceneEditorSection bind:scenes={packageData.scenes} gameId={packageData.project.id} />
			{:else if activeTab === "stateDefs"}
				<StateDefEditorSection bind:stateDefs={packageData.stateDefs} gameId={packageData.project.id} />
			{:else if activeTab === "characters"}
				<CharacterEditorSection bind:characters={packageData.characters} gameId={packageData.project.id} />
			{:else if activeTab === "maps"}
				<MapEditorSection
					bind:zones={packageData.zones}
					bind:maps={packageData.maps}
					bind:locations={packageData.locations}
					gameId={packageData.project.id}
				/>
			{:else if activeTab === "connections"}
				<LocationConnectionEditorSection
					bind:connections={packageData.locationConnections}
					locations={packageData.locations}
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
					gameId={packageData.project.id}
				/>
			{/if}
		</main>
	{/if}
</div>

<style>
	.editor-container {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background: #0f172a;
		color: #e2e8f0;
		font-family: inherit;
	}
	.loading-box,
	.error-box {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: #94a3b8;
		font-size: 1.1rem;
	}

	.editor-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem 1.5rem;
		background: #1e293b;
		border-bottom: 1px solid #334155;
	}
	.left-group {
		display: flex;
		align-items: center;
		gap: 1rem;
	}
	.left-group h2 {
		font-size: 1.25rem;
		margin: 0;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	.sub-id {
		font-size: 0.8rem;
		color: #64748b;
		font-weight: normal;
	}

	.back-btn {
		background: #334155;
		color: #e2e8f0;
		border: none;
		padding: 0.5rem 0.9rem;
		border-radius: 6px;
		cursor: pointer;
		font-weight: 600;
	}
	.back-btn:hover {
		background: #475569;
	}

	.save-btn {
		background: #3b82f6;
		color: white;
		border: none;
		padding: 0.6rem 1.2rem;
		border-radius: 6px;
		font-weight: 600;
		cursor: pointer;
	}
	.save-btn:hover {
		background: #2563eb;
	}
	.save-btn:disabled {
		background: #64748b;
		cursor: not-allowed;
	}

	.editor-tabs {
		display: flex;
		background: #1e293b;
		border-bottom: 1px solid #334155;
		padding: 0 1.5rem;
		gap: 0.5rem;
	}
	.editor-tabs button {
		background: transparent;
		border: none;
		color: #94a3b8;
		padding: 0.8rem 1rem;
		font-weight: 600;
		cursor: pointer;
		border-bottom: 2px solid transparent;
	}
	.editor-tabs button:hover {
		color: #f8fafc;
	}
	.editor-tabs button.active {
		color: #38bdf8;
		border-bottom-color: #38bdf8;
	}

	.editor-body {
		flex: 1;
		padding: 2rem;
		overflow-y: auto;
		max-width: 900px;
		width: 100%;
		margin: 0 auto;
	}

	.form-section {
		background: #1e293b;
		padding: 1.5rem;
		border-radius: 8px;
		border: 1px solid #334155;
		display: flex;
		flex-direction: column;
		gap: 1.2rem;
	}
	.form-section h3 {
		margin: 0 0 0.5rem 0;
		font-size: 1.1rem;
		color: #f8fafc;
		border-bottom: 1px solid #334155;
		padding-bottom: 0.5rem;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.field label {
		font-size: 0.85rem;
		font-weight: 600;
		color: #94a3b8;
	}
	.field input,
	.field textarea {
		background: #0f172a;
		border: 1px solid #334155;
		border-radius: 6px;
		padding: 0.6rem;
		color: #e2e8f0;
		font-size: 0.95rem;
	}
	.field input:focus,
	.field textarea:focus {
		outline: none;
		border-color: #38bdf8;
	}
	.code-font {
		font-family: monospace;
		font-size: 0.9rem;
	}
</style>
