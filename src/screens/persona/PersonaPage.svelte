<script lang="ts">
	import { handleDeletePersona, personas } from "$/lib/chat/personaStore.svelte";
	import { confirm } from "@tauri-apps/plugin-dialog";
	import { appSetting } from "$/lib/stores/settingStore.svelte";
	import Avatar from "$components/avatar.svelte";

	let loading = $state(false);
	let error = $state<string | null>(null);

	// ========================================
	// FORM
	// ========================================

	let name = $state("");
	let avatar = $state("");
	let description = $state("");
	let tags = $state<string[]>([]);
	let worldInfoIds = $state<string[]>([]);
	let color = $state("#6366f1");
	let isSystemDefault = $state(false);

	// ========================================
	// SELECT
	// ========================================

	async function selectPersona(personaId: string) {
		await appSetting.update({ selectedPersonaId: personaId });

		const persona = personas.activePersona;

		name = persona?.name ?? "";
		avatar = persona?.avatar ?? "";
		description = persona?.description ?? "";
		tags = persona?.tags ?? [];
		color = persona?.color ?? "#6366f1";
		isSystemDefault = persona?.isSystemDefault ?? false;
	}

	function settingDefault() {
		name = "";
		avatar = "";
		description = "";
		tags = [];
		color = "#6366f1";
		isSystemDefault = false;
	}
	// ========================================
	// SAVE
	// ========================================

	async function savePersona() {
		if (!name.trim()) {
			error = "이름을 입력해주세요.";

			return;
		}

		loading = true;
		error = null;

		const selectedPersona = personas.activePersona;

		try {
			const input = {
				name,
				avatar: avatar || "",
				description: description || "",
				tags: tags || [],
				worldInfoIds: worldInfoIds || [],
				color: color || "",
				isSystemDefault,
			};
			if (selectedPersona) {
				await personas.patch(selectedPersona.id, input);
			} else {
				await personas.create(input);
			}
		} catch (err) {
			error = String(err);
		} finally {
			loading = false;
		}
	}

	// ========================================
	// DELETE
	// ========================================

	async function removePersona() {
		const selectedPersona = personas.activePersona;
		if (!selectedPersona) return;

		const confirmed = await confirm(`"${selectedPersona.name}" 페르소나를 삭제할까요?`);
		if (!confirmed) return;
		loading = true;
		error = null;

		try {
			await handleDeletePersona(selectedPersona.id);
			settingDefault();
		} catch (err) {
			error = String(err);
			console.error(err);
		} finally {
			loading = false;
		}
	}
</script>

<div class="content persona-screen">
	<!-- ========================= -->
	<!-- SIDEBAR -->
	<!-- ========================= -->

	<aside class="persona-list">
		<div class="list-header">
			<h2>Personas</h2>

			<button onclick={() => personas.newPersona()}> + </button>
		</div>

		<div class="list-content">
			{#if loading && personas.value.length === 0}
				<p class="empty">Loading...</p>
			{:else if personas.value.length === 0}
				<p class="empty">등록된 페르소나가 없습니다.</p>
			{:else}
				{#each personas.value as persona}
					<button
						class:active={personas.activePersona?.id === persona.id}
						class="persona-item"
						onclick={() => selectPersona(persona.id)}
					>
						<Avatar avatar={persona.avatar ?? persona.color} />

						<div class="persona-info">
							<strong>
								{persona.name}
							</strong>

							{#if persona.description}
								<span>
									{persona.description}
								</span>
							{/if}
						</div>
					</button>
				{/each}
			{/if}
		</div>
	</aside>

	<!-- ========================= -->
	<!-- EDITOR -->
	<!-- ========================= -->

	<main class="persona-editor">
		<header class="editor-header">
			<h1>
				{personas.activePersona ? "Edit Persona" : "New Persona"}
			</h1>

			<div class="header-actions">
				{#if personas.activePersona}
					<button class="danger" onclick={() => removePersona()}> Delete </button>
				{/if}

				<button class="primary" onclick={() => savePersona()} disabled={loading}> Save </button>
			</div>
		</header>

		{#if error}
			<div class="error">
				{JSON.stringify(error)}
			</div>
		{/if}

		<div class="form">
			<label>
				<span>Name</span>

				<input bind:value={name} placeholder="Persona name" />
			</label>

			<label>
				<span>Avatar</span>

				<input bind:value={avatar} placeholder="Avatar path or URL" />
			</label>

			<label>
				<span>Description</span>

				<textarea bind:value={description} placeholder="Describe this persona"></textarea>
			</label>

			<label>
				<span>Tags</span>

				<input bind:value={tags} placeholder="friend, fantasy, assistant" />
			</label>

			<div class="form-row">
				<label>
					<span>Color</span>

					<input type="color" bind:value={color} />
				</label>

				<label class="checkbox">
					<input type="checkbox" bind:checked={isSystemDefault} />

					<span> System Default </span>
				</label>
			</div>
		</div>
	</main>
</div>

<style>
	.persona-screen {
		display: flex;
		width: 100%;
		height: 100%;
		min-height: 0;
		background: var(--background-primary);
	} /* =========================
       LIST
    ========================= */
	.persona-list {
		width: 280px;
		flex-shrink: 0;
		border-right: 1px solid var(--border-color);
		display: flex;
		flex-direction: column;
	}
	.list-header {
		height: 48px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 12px;
		border-bottom: 1px solid var(--border-color);
	}
	.list-header h2 {
		margin: 0;
		font-size: 15px;
	}

	.list-header button {
		border: none;
		background: transparent;
		font-size: 22px;
		color: inherit;
	}
	.list-content {
		overflow-y: auto;
	}
	.persona-item {
		width: 100%;
		display: flex;
		gap: 10px;
		align-items: center;
		padding: 10px 12px;
		border: none;
		background: transparent;
		color: inherit;
		text-align: left;
	}
	.persona-item:hover,
	.persona-item.active {
		background: var(--button-hover-bg);
	}
	.persona-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.persona-info span {
		font-size: 12px;
		opacity: 0.6;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	} /* =========================
       EDITOR
    ========================= */
	.persona-editor {
		flex: 1;
		min-width: 0;
		overflow-y: auto;
	}
	.editor-header {
		height: 56px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 20px;
		border-bottom: 1px solid var(--border-color);
	}
	.editor-header h1 {
		margin: 0;
		font-size: 16px;
	}
	.header-actions {
		display: flex;
		gap: 8px;
	}
	.header-actions button {
		padding: 6px 12px;
		border: 1px solid var(--border-color);
		background: transparent;
		color: inherit;
		border-radius: var(--card-radius);
	}
	.header-actions .primary {
		background: var(--accent-color);
		color: white;
		border-color: transparent;
	}
	.header-actions .danger {
		color: #e55;
	}
	.form {
		max-width: 700px;
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.form-row {
		display: flex;
		align-items: center;
		gap: 30px;
	}
	.checkbox {
		flex-direction: row;
		align-items: center;
	}
	.checkbox input {
		width: auto;
	}
	.error {
		margin: 16px 24px;
		padding: 10px;
		color: #f55;
		background: rgba(255, 0, 0, 0.08);
	}
	.empty {
		padding: 20px;
		opacity: 0.5;
	}
</style>
