<script lang="ts">
	import { worldInfos } from "$/lib/chat/worldInfoStore.svelte";
	import type { WorldInfoBook, WorldInfoEntry } from "$type/conv";
	// ========================================
	// DATA
	// ========================================

	let entries = $state<WorldInfoEntry[]>([]);
	let selectedBook = $state<WorldInfoBook | null>(null);
	let selectedEntry = $state<WorldInfoEntry | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);

	// ========================================
	// BOOK FORM
	// ========================================

	let bookName = $state("");
	let bookDescription = $state("");

	// ========================================
	// ENTRY FORM
	// ========================================

	let entryKeys: string[] = $state([]);
	let entrySecondaryKeys: string[] = $state([]);
	let entryContent = $state("");
	let entryEnabled = $state(true);
	let entryPriority = $state(0);
	let entryConstant: boolean = $state(false);

	// ========================================
	// LOAD ENTRIES
	// ========================================

	async function loadEntries(bookId: string) {
		loading = true;

		try {
			entries = await worldInfos.getWorldInfoEntries(bookId);
		} catch (err) {
			error = String(err);
		} finally {
			loading = false;
		}
	}

	// ========================================
	// SELECT BOOK
	// ========================================

	async function selectBook(book: WorldInfoBook) {
		selectedBook = book;
		selectedEntry = null;
		bookName = book.name;
		bookDescription = book.description ?? "";

		await loadEntries(book.id);
	}

	// ========================================
	// NEW BOOK
	// ========================================

	function newBook() {
		selectedBook = null;
		selectedEntry = null;
		entries = [];
		bookName = "";
		bookDescription = "";
	}

	// ========================================
	// SAVE BOOK
	// ========================================

	async function saveBook() {
		if (!bookName.trim()) {
			error = "Book 이름을 입력해주세요.";
			return;
		}

		loading = true;
		error = null;

		try {
			if (selectedBook) {
				await worldInfos.create({
					id: selectedBook.id,
					name: bookName,
					description: bookDescription || null,
				});
			} else {
				const book = await worldInfos.create({
					name: bookName,
					description: bookDescription || null,
				});

				selectedBook = book;
			}

			if (selectedBook) {
				await loadEntries(selectedBook.id);
			}
		} catch (err) {
			error = String(err);
		} finally {
			loading = false;
		}
	}

	// ========================================
	// DELETE BOOK
	// ========================================

	async function removeBook() {
		if (!selectedBook) return;

		if (!confirm(`"${selectedBook.name}" Book을 삭제할까요?`)) {
			return;
		}

		try {
			await worldInfos.remove(selectedBook.id);
			newBook();

		} catch (err) {
			error = String(err);
		}
	}

	// ========================================
	// SELECT ENTRY
	// ========================================

	function selectEntry(entry: WorldInfoEntry) {
		selectedEntry = entry;
		entryKeys = entry.keys ?? [];
		entrySecondaryKeys = entry.secondaryKeys ?? [];
		entryContent = entry.content ?? "";
		entryEnabled = entry.enabled;
		entryPriority = entry.priority;
		entryConstant = entry.constant ?? false;
	}

	// ========================================
	// NEW ENTRY
	// ========================================

	function newEntry() {
		selectedEntry = null;

		entryKeys = [];
		entrySecondaryKeys = [];
		entryContent = "";
		entryEnabled = true;
		entryPriority = 0;
		entryConstant = false;
	}

	// ========================================
	// SAVE ENTRY
	// ========================================

	async function saveEntry() {
		if (!selectedBook) {
			error = "먼저 Book을 선택해주세요.";

			return;
		}

		try {
			if (selectedEntry) {
				await worldInfos.upsertWorldInfoEntry({
					id: selectedEntry.id,
					bookId: selectedBook.id,
					keys: entryKeys || null,
					secondaryKeys: entrySecondaryKeys || null,
					content: entryContent || null,
					enabled: entryEnabled,
					priority: entryPriority,
					constant: entryConstant,
				});
			} else {
				await worldInfos.upsertWorldInfoEntry({
					bookId: selectedBook.id,

					keys: entryKeys || null,
					secondaryKeys: entrySecondaryKeys || null,
					content: entryContent || null,
					enabled: entryEnabled,
					priority: entryPriority,
					constant: entryConstant,
				});
			}

			await loadEntries(selectedBook.id);
		} catch (err) {
			error = String(err);
		}
	}

	// ========================================
	// DELETE ENTRY
	// ========================================

	async function removeEntry() {
		if (!selectedEntry) return;

		if (!confirm("Entry를 삭제할까요?")) {
			return;
		}

		try {
			await worldInfos.deleteWorldInfoEntry(selectedEntry.id);

			newEntry();

			if (selectedBook) {
				await loadEntries(selectedBook.id);
			}
		} catch (err) {
			error = String(err);
		}
	}
</script>

<div class="worldinfo-screen">
	<!-- ========================= -->
	<!-- BOOK LIST -->
	<!-- ========================= -->

	<aside class="book-list">
		<div class="sidebar-header">
			<h2>World Info</h2>

			<button onclick={() => newBook()}> + </button>
		</div>

		<div class="book-items">
			{#each worldInfos.value as book}
				<button class="book-item" class:active={selectedBook?.id === book.id} onclick={() => selectBook(book)}>
					<strong>
						{book.name}
					</strong>

					{#if book.description}
						<span>
							{book.description}
						</span>
					{/if}
				</button>
			{/each}
		</div>
	</aside>

	<!-- ========================= -->
	<!-- CONTENT -->
	<!-- ========================= -->

	<main class="worldinfo-content">
		{#if error}
			<div class="error">
				{error}
			</div>
		{/if}

		<!-- ========================= -->
		<!-- BOOK EDITOR -->
		<!-- ========================= -->

		<section class="book-editor">
			<header>
				<h1>
					{selectedBook ? "Edit World Info" : "New World Info"}
				</h1>

				<div>
					{#if selectedBook}
						<button class="danger" onclick={() => removeBook()}> Delete </button>
					{/if}

					<button class="primary" onclick={() => saveBook()}> Save </button>
				</div>
			</header>

			<label>
				<span>Name</span>

				<input bind:value={bookName} placeholder="World Info Book name" />
			</label>

			<label>
				<span>Description</span>

				<textarea bind:value={bookDescription} placeholder="Describe this world"></textarea>
			</label>
		</section>

		<!-- ========================= -->
		<!-- ENTRY SECTION -->
		<!-- ========================= -->

		{#if selectedBook}
			<section class="entries-section">
				<div class="entries-header">
					<h2>Entries</h2>

					<button onclick={() => newEntry()}> + New Entry </button>
				</div>

				<div class="entries-layout">
					<!-- ENTRY LIST -->

					<aside class="entry-list">
						{#each entries as entry}
							<button
								class="entry-item"
								class:active={selectedEntry?.id === entry.id}
								onclick={() => selectEntry(entry)}
							>
								<strong>
									{entry.keys || "Untitled"}
								</strong>

								<span>
									Priority:
									{entry.priority}
								</span>
							</button>
						{/each}
					</aside>

					<!-- ENTRY EDITOR -->

					<div class="entry-editor">
						<header>
							<h2>
								{selectedEntry ? "Edit Entry" : "New Entry"}
							</h2>

							<div>
								{#if selectedEntry}
									<button class="danger" onclick={() => removeEntry()}> Delete </button>
								{/if}

								<button class="primary" onclick={() => saveEntry()}> Save </button>
							</div>
						</header>

						<label>
							<span> Keys </span>

							<input bind:value={entryKeys} placeholder="dragon, wyvern" />
						</label>

						<label>
							<span> Secondary Keys </span>

							<input bind:value={entrySecondaryKeys} placeholder="fire, monster" />
						</label>

						<label>
							<span> Content </span>

							<textarea
								class="content-editor"
								bind:value={entryContent}
								placeholder="World information..."
							></textarea>
						</label>

						<div class="options">
							<label class="checkbox">
								<input type="checkbox" bind:checked={entryEnabled} />

								Enabled
							</label>

							<label>
								<span> Priority </span>

								<input type="number" bind:value={entryPriority} />
							</label>

							<label class="checkbox">
								<input type="checkbox" bind:checked={entryConstant} />

								Constant
							</label>
						</div>
					</div>
				</div>
			</section>
		{/if}
	</main>
</div>

<style>
	.worldinfo-screen {
		display: flex;

		height: 100%;

		min-height: 0;
	}

	/* =========================
       BOOK LIST
    ========================= */

	.book-list {
		width: 260px;

		flex-shrink: 0;

		border-right: 1px solid var(--border-color);

		display: flex;

		flex-direction: column;
	}

	.sidebar-header {
		height: 48px;

		display: flex;

		align-items: center;

		justify-content: space-between;

		padding: 0 12px;

		border-bottom: 1px solid var(--border-color);
	}

	.sidebar-header h2 {
		font-size: 15px;

		margin: 0;
	}

	button {
		cursor: pointer;
	}

	.sidebar-header button {
		border: none;

		background: transparent;

		color: inherit;

		font-size: 20px;
	}

	.book-items {
		overflow-y: auto;
	}

	.book-item {
		width: 100%;

		padding: 12px;

		border: none;

		background: transparent;

		color: inherit;

		text-align: left;

		display: flex;

		flex-direction: column;

		gap: 4px;
	}

	.book-item:hover,
	.book-item.active {
		background: var(--button-hover-bg);
	}

	.book-item span {
		font-size: 12px;

		opacity: 0.6;
	}

	/* =========================
       MAIN
    ========================= */

	.worldinfo-content {
		flex: 1;

		min-width: 0;

		overflow-y: auto;
	}

	.book-editor {
		padding: 20px;

		border-bottom: 1px solid var(--border-color);

		display: flex;

		flex-direction: column;

		gap: 16px;
	}

	section > header,
	.entries-header,
	.entry-editor header {
		display: flex;

		align-items: center;

		justify-content: space-between;
	}

	h1,
	h2 {
		margin: 0;
	}

	h1 {
		font-size: 17px;
	}

	h2 {
		font-size: 15px;
	}

	header div {
		display: flex;

		gap: 8px;
	}

	button.primary,
	button.danger,
	.entries-header button {
		padding: 6px 12px;

		border: 1px solid var(--border-color);

		border-radius: var(--card-radius);

		background: transparent;

		color: inherit;
	}

	button.primary {
		background: var(--accent-color);

		color: white;

		border-color: transparent;
	}

	button.danger {
		color: #e55;
	}

	label {
		display: flex;

		flex-direction: column;

		gap: 6px;
	}

	label > span {
		font-size: 12px;

		opacity: 0.7;
	}

	input,
	textarea {
		box-sizing: border-box;

		width: 100%;

		padding: 9px;

		border: 1px solid var(--input-border-color);

		background: var(--input-bg);

		color: inherit;

		border-radius: var(--card-radius);
	}

	textarea {
		min-height: 80px;

		resize: vertical;
	}

	/* =========================
       ENTRIES
    ========================= */

	.entries-section {
		padding: 20px;
	}

	.entries-header {
		margin-bottom: 15px;
	}

	.entries-layout {
		display: grid;

		grid-template-columns:
			220px
			minmax(0, 1fr);

		border: 1px solid var(--border-color);

		min-height: 450px;
	}

	.entry-list {
		border-right: 1px solid var(--border-color);
	}

	.entry-item {
		width: 100%;

		display: flex;

		flex-direction: column;

		gap: 4px;

		padding: 10px;

		border: none;

		background: transparent;

		color: inherit;

		text-align: left;
	}

	.entry-item:hover,
	.entry-item.active {
		background: var(--button-hover-bg);
	}

	.entry-item span {
		font-size: 11px;

		opacity: 0.5;
	}

	.entry-editor {
		padding: 16px;

		display: flex;

		flex-direction: column;

		gap: 16px;
	}

	.content-editor {
		min-height: 200px;

		font-family: inherit;
	}

	.options {
		display: flex;

		align-items: center;

		gap: 25px;
	}

	.checkbox {
		flex-direction: row;

		align-items: center;
	}

	.checkbox input {
		width: auto;
	}

	.error {
		margin: 15px;

		padding: 10px;

		color: #f55;

		background: rgba(255, 0, 0, 0.08);
	}

	@media (max-width: 700px) {
		.book-list {
			width: 180px;
		}

		.entries-layout {
			grid-template-columns: 1fr;
		}

		.entry-list {
			border-right: none;

			border-bottom: 1px solid var(--border-color);
		}
	}
</style>
