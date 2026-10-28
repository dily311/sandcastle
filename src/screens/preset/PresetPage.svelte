<script lang="ts">
	import { Plus, MoreVertical, SlidersHorizontal } from "@lucide/svelte";

	import { presets } from "$/data/aiModelList";

	let selected = $state(0);
	let preset = $derived(presets[selected]);
</script>

<div class="page">
	<div class="page-title">
		<div>
			<h2>프리셋</h2>
			<p>AI 대화에 사용할 설정을 저장하고 관리합니다.</p>
		</div>

		<button class="primary">
			<Plus size={16} />
			새 프리셋
		</button>
	</div>

	<div class="layout">
		<section class="preset-list">
			{#each presets as item, index}
				<button class:selected={selected === index} class="preset-card" onclick={() => (selected = index)}>
					<div class="preset-icon">
						<SlidersHorizontal size={18} />
					</div>

					<div class="preset-info">
						<strong>{item.name}</strong>
						<span>{item.description}</span>
					</div>

					<MoreVertical size={16} />
				</button>
			{/each}
		</section>

		<section class="editor">
			<div class="editor-header">
				<div>
					<h3>{preset.name}</h3>
					<p>{preset.description}</p>
				</div>

				<button class="delete"> 삭제 </button>
			</div>

			<div class="settings">
				<div class="setting">
					<div class="setting-title">
						<span>Temperature</span>
						<strong>{preset.temperature}</strong>
					</div>

					<input type="range" min="0" max="2" step="0.1" bind:value={preset.temperature} />

					<div class="range-labels">
						<span>정확함</span>
						<span>창의적</span>
					</div>
				</div>

				<div class="setting">
					<div class="setting-title">
						<span>Top P</span>
						<strong>{preset.topP}</strong>
					</div>

					<input type="range" min="0" max="1" step="0.05" bind:value={preset.topP} />

					<div class="range-labels">
						<span>낮음</span>
						<span>높음</span>
					</div>
				</div>

				<label>
					<span>System Prompt</span>
					<textarea rows="8" placeholder="AI에게 전달할 시스템 프롬프트를 입력하세요."></textarea>
				</label>

				<button class="save"> 변경사항 저장 </button>
			</div>
		</section>
	</div>
</div>

<style>
	.page-title {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 18px;
	}

	h2 {
		margin: 0;
		font-size: 19px;
		color: var(--text-color);
	}

	.page-title p {
		margin: 5px 0 0;
		font-size: 11px;
		color: var(--text-muted);
	}

	.primary,
	.save {
		height: 36px;
		padding: 0 13px;

		display: flex;
		align-items: center;
		gap: 6px;

		border: 0;
		border-radius: 8px;

		background: var(--accent-color);
		color: white;

		font-size: 11px;
		cursor: pointer;
	}

	.layout {
		display: grid;
		grid-template-columns: 310px 1fr;
		gap: 14px;
		align-items: start;
	}

	.preset-list {
		padding: 7px;

		background: var(--card-bg);
		border: 1px solid var(--card-border-color);
		border-radius: 14px;
	}

	.preset-card {
		width: 100%;
		min-height: 65px;
		padding: 9px;

		display: flex;
		align-items: center;
		gap: 10px;

		border: 1px solid transparent;
		border-radius: 10px;

		background: transparent;
		text-align: left;

		color: var(--text-muted);
		cursor: pointer;
	}

	.preset-card:hover {
		background: var(--card-hover-bg);
	}

	.preset-card.selected {
		border-color: #e2e0fb;
		/* background: #f8f8ff; */
	}

	.preset-icon {
		width: 36px;
		height: 36px;

		display: flex;
		align-items: center;
		justify-content: center;

		border-radius: 9px;

		background: #efeeff;
		color: #766ef0;
	}

	.preset-info {
		flex: 1;
	}

	.preset-info strong {
		display: block;
		color: var(--text-color);
		font-size: 12px;
	}

	.preset-info span {
		display: block;
		margin-top: 3px;
		color: var(--text-muted);
		font-size: 10px;
	}

	.editor {
		border: 1px solid var(--card-border-color);
		border-radius: 14px;
		background: var(--background-secondary);
	}

	.editor-header {
		padding: 17px;

		display: flex;
		justify-content: space-between;

		border-bottom: 1px solid var(--border-color);
	}

	.editor-header p {
		margin: 4px 0 0;
		font-size: 10px;
		color: var(--text-muted);
	}

	.delete {
		border: 0;
		background: transparent;
		color: var(--color-red);
		font-size: 10px;
		cursor: pointer;
	}

	.settings {
		padding: 20px;
	}

	.setting {
		margin-bottom: 23px;
	}

	.setting-title {
		display: flex;
		justify-content: space-between;
		margin-bottom: 10px;
	}

	.setting-title span {
		color: var(--text-muted);
		font-size: 11px;
	}

	.setting-title strong {
		color: #776ff0;
		font-size: 11px;
	}

	.range-labels {
		display: flex;
		justify-content: space-between;
		margin-top: 4px;

		color: var(--text-faint);
		font-size: 9px;
	}

	label > span {
		display: block;
		margin-bottom: 7px;
		/* color: #555a63; */
		font-size: 11px;
	}

	textarea {
		width: 100%;

		resize: vertical;

		padding: 10px;

		border: 1px solid var(--card-border-color);
		border-radius: 8px;

		outline: none;
		color: var(--text-faint);

		font-family: inherit;
		font-size: 11px;
		line-height: 1.6;
	}

	.save {
		margin-top: 18px;
		margin-left: auto;
	}
</style>
