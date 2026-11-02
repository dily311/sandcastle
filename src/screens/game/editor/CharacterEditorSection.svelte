<script lang="ts">
	import type { GameCharactersInput } from "$/lib/type/game";

	type Props = {
		characters: GameCharactersInput[];
		gameId: string;
	};
	let { characters = $bindable([]), gameId }: Props = $props();

	let selectedIndex = $state<number | null>(characters.length > 0 ? 0 : null);
	let activeChar = $derived(selectedIndex !== null && characters[selectedIndex] ? characters[selectedIndex] : null);

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

	function handleDeleteCharacter(index: number) {
		if (!confirm(`"${characters[index].name}" 캐릭터를 삭제하시겠습니까?`)) return;
		characters.splice(index, 1);
		selectedIndex = characters.length > 0 ? Math.max(0, index - 1) : null;
	}
</script>

<div class="sub-editor-layout">
	<!-- 좌측: 캐릭터 목록 -->
	<div class="item-list-pane">
		<div class="pane-header">
			<h4>캐릭터 목록 ({characters.length})</h4>
			<button class="add-btn" onclick={handleAddCharacter}>+ 캐릭터 추가</button>
		</div>
		<div class="scroll-list">
			{#each characters as char, index (char.id || index)}
				<div class="list-item" class:active={selectedIndex === index} onclick={() => (selectedIndex = index)}>
					<div class="item-info">
						<span class="title">{char.name}</span>
					</div>
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
				<button class="delete-btn" onclick={() => handleDeleteCharacter(selectedIndex!)}>삭제</button>
			</div>
			<div class="form-grid">
				<div class="field-row-split">
					<div class="field">
						<label>캐릭터 이름 (Name)</label>
						<input type="text" bind:value={activeChar.name} />
					</div>
				</div>
				<div class="field">
					<label>아바타 이미지 경로/URL (Avatar)</label>
					<input type="text" bind:value={activeChar.avatar} placeholder="https:// 또는 파일 경로" />
				</div>
				<div class="field">
					<label>설명 (Description / Persona)</label>
					<textarea bind:value={activeChar.description} rows="3" placeholder="캐릭터의 배경 설정 및 성격"
					></textarea>
				</div>
				<div class="field">
					<label>첫 대사 (First Message)</label>
					<textarea bind:value={activeChar.firstMessage} rows="2" placeholder="채팅 시작 시 건네는 첫 인사"
					></textarea>
				</div>
				<div class="field">
					<label>행동 타입 (Behavior Type)</label>
					<input type="text" bind:value={activeChar.behaviorType} placeholder="예: npc, companion 등" />
				</div>
				<div class="field">
					<label>예시 대화 (Example Dialogue)</label>
					<textarea
						bind:value={activeChar.exampleDialogue}
						rows="3"
						placeholder="<START>\n"></textarea>
				</div>
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
		gap: 1.5rem;
		height: 100%;
		min-height: 500px;
	}
	.item-list-pane,
	.item-detail-pane {
		background: #1e293b;
		border: 1px solid #334155;
		border-radius: 8px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.pane-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		border-bottom: 1px solid #334155;
		background: #0f172a;
	}
	.pane-header h4 {
		margin: 0;
		font-size: 0.95rem;
		color: #f8fafc;
	}
	.add-btn {
		background: #3b82f6;
		color: white;
		border: none;
		padding: 0.4rem 0.8rem;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}
	.add-btn:hover {
		background: #2563eb;
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
		background: #0f172a;
		border-radius: 6px;
		cursor: pointer;
		border: 1px solid transparent;
		transition: all 0.15s;
	}
	.list-item:hover {
		border-color: #475569;
	}
	.list-item.active {
		border-color: #38bdf8;
		background: #172554;
	}
	.item-info {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	.item-info .title {
		font-weight: 600;
		font-size: 0.9rem;
		color: #f8fafc;
	}
	.item-info .key {
		font-size: 0.75rem;
		color: #94a3b8;
	}
	.empty-text {
		text-align: center;
		color: #64748b;
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
		border-bottom: 1px solid #334155;
		padding-bottom: 0.8rem;
	}
	.detail-header h3 {
		margin: 0;
		font-size: 1.1rem;
		color: #f8fafc;
	}
	.delete-btn {
		background: #ef4444;
		color: white;
		border: none;
		padding: 0.4rem 0.8rem;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}
	.delete-btn:hover {
		background: #dc2626;
	}
	.form-grid {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.field-row-split {
		display: grid;
		grid-template-columns: 1fr 1fr;
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
	.no-selection {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: #64748b;
		font-size: 0.95rem;
		text-align: center;
	}
</style>
