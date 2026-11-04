<script lang="ts">
	import type { PackageWorldInfoBook } from "$/lib/type/game";

	let { worldInfoBooks = $bindable([]) }: { worldInfoBooks: PackageWorldInfoBook[] } = $props();

	let selectedBookIndex = $state<number | null>(worldInfoBooks.length > 0 ? 0 : null);
	let selectedEntryIndex = $state<number | null>(null);

	let activeBook = $derived(
		selectedBookIndex !== null && worldInfoBooks[selectedBookIndex] ? worldInfoBooks[selectedBookIndex] : null,
	);
	let activeEntry = $derived(
		selectedEntryIndex !== null && activeBook && activeBook.entries[selectedEntryIndex]
			? activeBook.entries[selectedEntryIndex]
			: null,
	);

	// --- 북(Book) 관리 핸들러 ---
	function handleAddBook() {
		const newBook: PackageWorldInfoBook = {
			id: crypto.randomUUID(),
			book: {
				id: crypto.randomUUID(),
				name: "새로운 월드 인포 북",
				description: "",
			},
			entries: [],
		};
		worldInfoBooks.push(newBook);
		selectedBookIndex = worldInfoBooks.length - 1;
		selectedEntryIndex = null;
	}

	function handleDeleteBook(index: number) {
		if (!confirm(`"${worldInfoBooks[index].book.name}" 북을 삭제하시겠습니까? 하위 엔트리도 함께 삭제됩니다.`))
			return;
		worldInfoBooks.splice(index, 1);
		selectedBookIndex = worldInfoBooks.length > 0 ? 0 : null;
		selectedEntryIndex = null;
	}

	// --- 엔트리(Entry) 관리 핸들러 ---
	function handleAddEntry() {
		if (!activeBook) return;
		const newEntry = {
			id: crypto.randomUUID(),
			bookId: activeBook.book.id || activeBook.id,
			keys: ["키워드1", "키워드2"],
			secondaryKeys: [],
			content: "여기에 설정이나 lore 내용을 작성하세요...",
			enabled: true,
			priority: 10,
			constant: false,
		};
		activeBook.entries.push(newEntry);
		selectedEntryIndex = activeBook.entries.length - 1;
	}

	function handleDeleteEntry(index: number) {
		if (!activeBook || !confirm("이 엔트리를 삭제하시겠습니까?")) return;
		activeBook.entries.splice(index, 1);
		selectedEntryIndex = activeBook.entries.length > 0 ? Math.max(0, index - 1) : null;
	}

	// 쉼표(,)로 구분된 키워드 문자열을 배열로 변환/역변환 처리 헬퍼
	function keysToString(keys: string[]): string {
		return keys ? keys.join(", ") : "";
	}

	function stringToKeys(str: string): string[] {
		return str
			.split(",")
			.map((k) => k.trim())
			.filter((k) => k.length > 0);
	}
</script>

<div class="world-info-layout">
	<!-- 1단: 월드 인포 북 목록 -->
	<div class="column-pane">
		<div class="pane-header">
			<h4>월드 인포 북 ({worldInfoBooks.length})</h4>
			<button class="add-btn" onclick={handleAddBook}>+ 북 추가</button>
		</div>
		<div class="scroll-list">
			{#each worldInfoBooks as item, idx (item.id)}
				<div
					role="none"
					class="list-item"
					class:active={selectedBookIndex === idx}
					onclick={() => {
						selectedBookIndex = idx;
						selectedEntryIndex = null;
					}}
				>
					<div class="item-info">
						<span class="title">{item.book.name}</span>
						<span class="meta">엔트리: {item.entries.length}개</span>
					</div>
					<button class="icon-del" onclick={() => handleDeleteBook(idx)}>×</button>
				</div>
			{:else}
				<div class="empty-text">등록된 북이 없습니다.</div>
			{/each}
		</div>
	</div>

	<!-- 2단: 선택된 북에 속한 엔트리 목록 -->
	<div class="column-pane">
		<div class="pane-header">
			<h4>항목 (Entries)</h4>
			<button class="add-btn" onclick={handleAddEntry} disabled={!activeBook}>+ 엔트리 추가</button>
		</div>
		<div class="scroll-list">
			{#if activeBook}
				{#each activeBook.entries as entry, idx (entry.id)}
					<div
						role="none"
						class="list-item"
						class:active={selectedEntryIndex === idx}
						onclick={() => (selectedEntryIndex = idx)}
					>
						<div class="item-info">
							<span class="title">{entry.keys[0] || "무제 키워드"}</span>
							<span class="meta">우선순위: {entry.priority}</span>
						</div>
						<button class="icon-del" onclick={() => handleDeleteEntry(idx)}>×</button>
					</div>
				{:else}
					<div class="empty-text">엔트리가 없습니다.</div>
				{/each}
			{:else}
				<div class="empty-text">북을 먼저 선택해주세요.</div>
			{/if}
		</div>
	</div>

	<!-- 3단: 상세 편집 패널 (북 설정 또는 엔트리 상세) -->
	<div class="detail-pane">
		{#if activeEntry && selectedEntryIndex !== null}
			<div class="detail-header">
				<h3>엔트리 상세 편집</h3>
				<span class="sub-tag">ID: {activeEntry.id}</span>
			</div>
			<div class="form-grid">
				<div class="field">
					<label for="book_triger">트리거 키워드 (쉼표로 구분)</label>
					<input
						id="book_triger"
						type="text"
						value={keysToString(activeEntry.keys)}
						oninput={(e) => (activeEntry.keys = stringToKeys(e.currentTarget.value))}
						placeholder="예: 엘프, 숲, 마법사"
					/>
				</div>

				<div class="field-row-split">
					<div class="field">
						<label for="book_priority">우선순위 (Priority)</label>
						<input id="book_priority" type="number" bind:value={activeEntry.priority} />
					</div>
					<div class="field checkbox-field">
						<label class="checkbox-label">
							<input type="checkbox" bind:checked={activeEntry.constant} />
							<span>상시 적용 (Constant)</span>
						</label>
						<label class="checkbox-label">
							<input type="checkbox" bind:checked={activeEntry.enabled} />
							<span>활성화 (Enabled)</span>
						</label>
					</div>
				</div>

				<div class="field">
					<label for="book_content">컨텐츠 내용 (Content / Lore)</label>
					<textarea
						id="book_content"
						bind:value={activeEntry.content}
						rows="8"
						placeholder="AI 프롬프트에 주입될 Lore 내용을 작성하세요..."></textarea>
				</div>
			</div>
		{:else if activeBook}
			<div class="detail-header">
				<h3>월드 인포 북 설정: <strong>{activeBook.book.name}</strong></h3>
			</div>
			<div class="form-grid">
				<div class="field">
					<label for="book_name">북 이름 (Book Name)</label>
					<input id="book_name" type="text" bind:value={activeBook.book.name} />
				</div>
				<div class="field">
					<label for="book_des">북 설명 (Description)</label>
					<textarea
						id="book_des"
						bind:value={activeBook.book.description}
						rows="4"
						placeholder="이 인포 북에 대한 설명을 입력하세요..."></textarea>
				</div>
			</div>
		{:else}
			<div class="no-selection">왼쪽에서 월드 인포 북이나 엔트리를 선택해 편집하세요.</div>
		{/if}
	</div>
</div>

<style>
	.world-info-layout {
		display: grid;
		grid-template-columns: 260px 240px 1fr;
		gap: 1rem;
		height: 100%;
		min-height: 550px;
	}

	.column-pane,
	.detail-pane {
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
		padding: 0.8rem;
		border-bottom: 1px solid #334155;
		background: #0f172a;
	}

	.pane-header h4 {
		margin: 0;
		font-size: 0.85rem;
		color: #f8fafc;
	}

	.add-btn {
		background: #3b82f6;
		color: white;
		border: none;
		padding: 0.3rem 0.6rem;
		border-radius: 4px;
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
	}
	.add-btn:disabled {
		background: #475569;
		opacity: 0.6;
		cursor: not-allowed;
	}
	.add-btn:not(:disabled):hover {
		background: #2563eb;
	}

	.scroll-list {
		flex: 1;
		overflow-y: auto;
		padding: 0.4rem;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}

	.list-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.6rem 0.8rem;
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
		gap: 0.1rem;
		overflow: hidden;
	}
	.item-info .title {
		font-weight: 600;
		font-size: 0.85rem;
		color: #f8fafc;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.item-info .meta {
		font-size: 0.7rem;
		color: #94a3b8;
	}

	.icon-del {
		background: transparent;
		border: none;
		color: #94a3b8;
		font-size: 1rem;
		cursor: pointer;
		padding: 0 0.3rem;
		border-radius: 4px;
	}
	.icon-del:hover {
		background: #ef4444;
		color: white;
	}

	.empty-text {
		text-align: center;
		color: #64748b;
		padding: 2rem 1rem;
		font-size: 0.8rem;
	}

	.detail-pane {
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
		font-size: 1rem;
		color: #f8fafc;
	}
	.sub-tag {
		font-size: 0.75rem;
		color: #64748b;
		font-family: monospace;
	}
	.detail-header strong {
		color: #38bdf8;
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

	.checkbox-field {
		display: flex;
		flex-direction: row;
		gap: 1.5rem;
		align-items: center;
	}
	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		font-size: 0.85rem;
		color: #e2e8f0;
	}

	.no-selection {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: #64748b;
		font-size: 0.9rem;
		text-align: center;
	}
</style>
