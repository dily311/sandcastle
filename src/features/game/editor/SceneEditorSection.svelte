<script lang="ts">
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	/** 지우기 전에 앱 확인 창으로 물어봄 */
	const ask = (message: string) => confirmAction({ title: "삭제", message, confirmLabel: "삭제", danger: true });
	import type { GameSceneInput } from "$features/game/types";
	import { Plus, Search, X } from "@lucide/svelte";
	import { IconButton } from "$/shared/ui";
	import Input from "$/shared/ui/Input.svelte";

	type Props = {
		scenes: GameSceneInput[];
		gameId: string;
	};
	let { scenes = $bindable([]), gameId }: Props = $props();

	let selectedIndex = $state<number | null>(scenes.length > 0 ? 0 : null);
	let searchQuery = $state("");
	let filteredScenes = $derived(
		scenes
			.map((scene, index) => ({ scene, index }))
			.filter(({ scene }) =>
				`${scene.sceneName} ${scene.sceneKey}`.toLowerCase().includes(searchQuery.trim().toLowerCase()),
			),
	);

	let activeScene = $derived(selectedIndex !== null && scenes[selectedIndex] ? scenes[selectedIndex] : null);

	/** 씬 설정은 JSON 문자열로 저장된다 */
	function sceneSetting(raw: string | null): Record<string, unknown> {
		try {
			return raw ? (JSON.parse(raw) as Record<string, unknown>) : {};
		} catch {
			return {};
		}
	}

	function handleAddScene() {
		const newScene: GameSceneInput = {
			id: crypto.randomUUID(), // 고유 ID 생성 (UUID v4)
			gameId: gameId,
			sceneName: "새로운 씬",
			description: "",
			sceneKey: `scene_${Date.now()}`,
			settingsJson: null,
			script: "",
			choices: [],
			isEnding: false,
			sortOrder: scenes.length,
			createdAt: Date.now(),
			updatedAt: Date.now(),
		};
		scenes.push(newScene);
		selectedIndex = scenes.length - 1;
	}

	async function handleDeleteScene(index: number) {
		if (!(await ask(`"${scenes[index].sceneName}" 씬을 삭제하시겠습니까?`))) return;
		scenes.splice(index, 1);
		selectedIndex = scenes.length > 0 ? Math.max(0, index - 1) : null;
	}

	function handleAddChoice() {
		if (!activeScene) return;
		activeScene.choices ??= [];
		activeScene.choices.push({
			id: crypto.randomUUID(),
			sceneId: activeScene.id,
			text: "새 선택지",
			nextSceneId: null,
			condition: null,
			effect: null,
			sortOrder: activeScene.choices.length,
		});
	}

	function handleDeleteChoice(choiceIndex: number) {
		activeScene?.choices?.splice(choiceIndex, 1);
	}

	function handleSearchInput(query: string) {
		searchQuery = query;
		const normalized = query.trim().toLowerCase();
		const match = scenes.findIndex((scene) =>
			`${scene.sceneName} ${scene.sceneKey}`.toLowerCase().includes(normalized),
		);
		if (
			selectedIndex === null ||
			!`${scenes[selectedIndex]?.sceneName ?? ""} ${scenes[selectedIndex]?.sceneKey ?? ""}`
				.toLowerCase()
				.includes(normalized)
		) {
			selectedIndex = match >= 0 ? match : null;
		}
	}
</script>

<div class="sub-editor-layout">
	<!-- 좌측: 씬 목록 -->
	<div class="item-list-pane">
		<div class="pane-header">
			<Input bind:value={searchQuery} placeholder="씬 검색" aria-label="씬 검색">
				{#snippet leading()}<Search />{/snippet}
			</Input>
			<IconButton label="씬 추가" size="md" onclick={handleAddScene}><Plus /></IconButton>
		</div>
		<div class="scroll-list">
			{#each filteredScenes as row (row.scene.id)}
				{@const ev = row.scene}
				{@const idx = row.index}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="list-item" class:active={selectedIndex === idx} onclick={() => (selectedIndex = idx)}>
					<div class="item-info">
						<span class="title">
							{ev.sceneName}
							{#if ev.isEnding}
								<span class="badge ending">엔딩</span>
							{/if}
						</span>
						<span class="key"><code>{ev.sceneKey}</code></span>
					</div>
					<IconButton class="icon-del" size="sm" label="씬 삭제" onclick={() => handleDeleteScene(idx)}
						><X /></IconButton
					>
				</div>
			{:else}
				<div class="empty-text">{searchQuery ? "검색 결과가 없습니다." : "등록된 씬이 없습니다."}</div>
			{/each}
		</div>
	</div>

	<!-- 우측: 선택된 씬 상세 편집 폼 -->
	<div class="item-detail-pane">
		{#if activeScene && selectedIndex !== null}
			<div class="detail-header">
				<h3>씬 상세 편집</h3>
			</div>
			<div class="form-grid">
				<details class="editor-accordion" open>
					<summary>씬 정보</summary>
					<div class="editor-accordion-content">
						<div class="field">
							<label for="scene_name">씬 이름 (Scene Name)</label>
							<input id="scene_name" type="text" bind:value={activeScene.sceneName} />
						</div>
						<div class="field">
							<label for="scene_key">씬 고유 키 (Scene Key)</label>
							<input id="scene_key" type="text" bind:value={activeScene.sceneKey} />
						</div>
						<div class="field">
							<label for="scene_des">설명 (Description)</label>
							<textarea id="scene_des" bind:value={activeScene.description} rows="4"></textarea>
						</div>
					</div>
				</details>
				<details class="editor-accordion scene-script-field" open>
					<summary>씬 텍스트 / 대본</summary>
					<div class="editor-accordion-content">
						<div class="field">
							<div class="script-label">
								<label for="scene_script">대사 입력</label><span
									>한 줄마다 내레이션 또는 대사를 입력합니다</span
								>
							</div>
							<textarea
								id="scene_script"
								class="scene-script"
								bind:value={activeScene.script}
								rows="12"
								placeholder={`골목은 비에 젖어 조용했다.
유나: 여기서 기다리고 있었어.
<emotion::유나>happy</emotion>`}></textarea>
							<p class="script-help">
								<code>캐릭터 이름: 대사</code> 형식으로 대사를 구분하고, 연출 태그도 함께 작성할 수
								있습니다. AI 캐릭터 응답의 선택지는 <code>&lt;choice&gt;문구&lt;/choice&gt;</code>로
								표현하며, 제작자 씬 선택지는 아래에서 별도로 편집합니다.
							</p>
						</div>
					</div>
				</details>
				<details class="editor-accordion scene-choices" open>
					<summary>씬 선택지 ({activeScene.choices?.length ?? 0})</summary>
					<div class="editor-accordion-content">
						<div class="choices-heading">
							<div><p>게임 화면에 버튼으로 표시되며, 대본과 별도로 저장됩니다.</p></div>
							<button class="add-btn" onclick={handleAddChoice}>+ 선택지 추가</button>
						</div>
						{#if activeScene.choices?.length}
							<div class="choice-list">
								{#each activeScene.choices as choice, choiceIndex (choice.id)}
									<article class="choice-card">
										<span class="choice-number">{String(choiceIndex + 1).padStart(2, "0")}</span>
										<div class="choice-fields">
											<label
												>선택지 텍스트<input
													bind:value={choice.text}
													placeholder="플레이어에게 보여줄 선택지"
												/></label
											><label
												>이동할 씬<select bind:value={choice.nextSceneId}
													><option value={null}>다음 씬 없음</option
													>{#each scenes as targetScene (targetScene.id)}<option
															value={targetScene.id}>{targetScene.sceneName}</option
														>{/each}</select
												></label
											>
										</div>
										<button
											class="choice-remove"
											aria-label="선택지 삭제"
											onclick={() => handleDeleteChoice(choiceIndex)}>×</button
										>
									</article>
								{/each}
							</div>
						{:else}<div class="choice-empty">
								선택지가 없습니다. 선택지를 추가하면 플레이어에게 버튼으로 표시됩니다.
							</div>{/if}
					</div>
				</details>
				<details class="editor-accordion" open>
					<summary>씬 설정</summary>
					<div class="editor-accordion-content">
						<div class="field-row">
							<label class="checkbox-label">
								<input type="checkbox" bind:checked={activeScene.isEnding} />
								<span>엔딩 씬으로 표시</span>
							</label>
						</div>
						<div class="field-row">
							<label class="checkbox-label">
								<input
									type="checkbox"
									checked={sceneSetting(activeScene.settingsJson).planUsable === true}
									onchange={(e) => (activeScene!.settingsJson = JSON.stringify({ ...sceneSetting(activeScene!.settingsJson), planUsable: e.currentTarget.checked }))} />
								<span>각본가가 무대에 올려도 됨</span>
							</label>
						</div>
						<div class="field">
							<label for="scene_sort">정렬 순서 (Sort Order)</label>
							<input id="scene_sort" type="number" bind:value={activeScene.sortOrder} />
						</div>
					</div>
				</details>
			</div>
		{:else}
			<div class="no-selection">편집할 씬을 선택하거나 새로 추가해주세요.</div>
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

	.badge.ending {
		background: var(--success);
		color: var(--surface);
		font-size: 0.65rem;
		font-weight: 700;
		padding: 0.1rem 0.4rem;
		border-radius: 4px;
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
	.field-row {
		display: flex;
		align-items: center;
		margin-top: 0.2rem;
	}
	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		font-size: 0.9rem;
		color: var(--text);
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
	.script-label {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: 0.5rem;
	}
	.script-label span,
	.script-help {
		color: var(--text-muted);
		font-size: 0.75rem;
	}
	.scene-script {
		min-height: 230px;
		font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
		line-height: 1.65;
		tab-size: 2;
	}
	.script-help {
		margin: 0;
	}
	.script-help code {
		color: var(--accent);
	}
	.scene-choices {
		margin-top: 0.3rem;
		padding-top: 1.1rem;
		border-top: 1px solid var(--border);
	}
	.choices-heading {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.8rem;
		margin-bottom: 0.8rem;
	}
	.choices-heading p {
		margin: 0.25rem 0 0;
		color: var(--text-muted);
		font-size: 0.76rem;
	}
	.choice-list {
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
	}
	.choice-card {
		display: grid;
		grid-template-columns: 34px minmax(0, 1fr) 28px;
		align-items: center;
		gap: 0.7rem;
		padding: 0.7rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-2);
	}
	.choice-number {
		color: var(--accent);
		font:
			600 0.75rem ui-monospace,
			monospace;
	}
	.choice-fields {
		display: grid;
		grid-template-columns: 1.3fr 1fr;
		gap: 0.7rem;
	}
	.choice-fields label {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		color: var(--text-muted);
		font-size: 0.72rem;
		font-weight: 600;
	}
	.choice-fields input,
	.choice-fields select {
		min-width: 0;
		padding: 0.5rem 0.6rem;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.8rem;
	}
	.choice-remove {
		border: 0;
		background: transparent;
		color: var(--text-muted);
		font-size: 1.2rem;
		cursor: pointer;
	}
	.choice-empty {
		padding: 0.9rem;
		border: 1px dashed var(--border-strong);
		border-radius: var(--radius-md);
		color: var(--text-muted);
		font-size: 0.8rem;
		text-align: center;
	}
	@media (max-width: 640px) {
		.choice-fields {
			grid-template-columns: 1fr;
		}
		.choices-heading {
			align-items: flex-start;
			flex-direction: column;
		}
	}
</style>
