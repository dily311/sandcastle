<script lang="ts">
  import type { SceneDraft } from './draft';
  import { createEmptyLineNode } from './draft';
  import { validateScene, type ValidationError } from './validateScene';
  import NodeCard from './NodeCard.svelte';
  import type { CharacterOption } from '../db/rowTypes';

  interface Props {
    scene: SceneDraft;
    variableKeys: string[];
    characterOptions: CharacterOption[];
    onSave: () => Promise<void>;
  }

  let { scene, variableKeys, characterOptions, onSave }: Props = $props();

  const knownVariables = $derived(new Set(variableKeys));
  const errors = $derived(validateScene(scene, knownVariables));
  const blockingCount = $derived(errors.filter((e) => e.severity === 'error').length);
  const warningCount = $derived(errors.filter((e) => e.severity === 'warning').length);

  let saveStatus = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');

  function errorsFor(nodeId: string): ValidationError[] {
    return errors.filter((e) => e.nodeId === nodeId);
  }

  function addNode() {
    const existingKeys = scene.nodes.map((n) => n.nodeKey);
    scene.nodes.push(createEmptyLineNode(scene.nodes.length, existingKeys));
  }

  function removeNode(nodeId: string) {
    const index = scene.nodes.findIndex((n) => n.id === nodeId);
    if (index !== -1) scene.nodes.splice(index, 1);
  }

  function scrollToNode(nodeId: string) {
    const el = document.getElementById(`node-${nodeId}`);
    el?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    el?.classList.add('flash');
    setTimeout(() => el?.classList.remove('flash'), 900);
  }

  async function handleSave() {
    saveStatus = 'saving';
    try {
      await onSave();
      saveStatus = 'saved';
    } catch (err) {
      console.error(err);
      saveStatus = 'error';
    }
  }
</script>

<div class="scene-editor">
  {#if errors.length > 0}
    <div class="error-summary">
      {#if blockingCount > 0}
        <p class="summary-line error">⚠ 저장 전 수정이 필요한 오류 {blockingCount}건</p>
      {/if}
      {#if warningCount > 0}
        <p class="summary-line warning">경고 {warningCount}건 (저장은 가능해요)</p>
      {/if}
      <ul>
        {#each errors as err}
          <li>
            <button class="error-link {err.severity}" onclick={() => err.nodeId && scrollToNode(err.nodeId)}>
              {err.message}
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <div class="node-list">
    {#each scene.nodes as node (node.id)}
      <NodeCard
        {node}
        allNodes={scene.nodes}
        {variableKeys}
        {characterOptions}
        errors={errorsFor(node.id)}
        onDelete={() => removeNode(node.id)}
      />
    {/each}
  </div>

  <button class="add-node-btn" onclick={addNode}>+ 새 노드 추가</button>

  <div class="save-bar">
    <button class="save-btn" disabled={blockingCount > 0 || saveStatus === 'saving'} onclick={handleSave}>
      {saveStatus === 'saving' ? '저장 중...' : '저장'}
    </button>
    {#if blockingCount > 0}
      <span class="save-hint">오류를 먼저 해결해주세요 ({blockingCount}건)</span>
    {:else if saveStatus === 'saved'}
      <span class="save-hint saved">저장됐어요.</span>
    {:else if saveStatus === 'error'}
      <span class="save-hint error">저장에 실패했어요. 콘솔을 확인해주세요.</span>
    {/if}
  </div>
</div>
<style>
	.scene-editor {
		padding: 1rem;
		overflow-y: scroll;
		max-width: 640px;
		font-family: system-ui, sans-serif;
	}

	.error-summary {
		background: var(--card-bg);
		border: 1px solid var(--card-border-color);
		border-radius: 8px;
		padding: 10px 14px;
		margin-bottom: 16px;
	}

	.summary-line {
		margin: 0 0 6px;
		font-weight: 600;
	}
	.summary-line.error {
		color: var(--color-red);
	}
	.summary-line.warning {
		color: var(--color-orange);
	}

	.error-summary ul {
		margin: 0;
		padding-left: 0;
		list-style: none;
	}

	.error-link {
		background: none;
		border: none;
		padding: 2px 0;
		text-align: left;
		cursor: pointer;
		font-size: 13px;
		text-decoration: underline;
	}
	.error-link.error {
		color: var(--color-red);
	}
	.error-link.warning {
		color: var(--color-orange);
	}

	.add-node-btn {
		display: block;
		width: 100%;
		padding: 10px;
		background: var(--card-bg);
		border: 1px dashed rgba(0, 0, 0, 0.25);
		border-radius: 8px;
		cursor: pointer;
		margin-bottom: 20px;
	}

	.save-bar {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.save-btn {
		padding: 10px 20px;
		border-radius: 6px;
		border: none;
		background: var(--color-green);
		color: white;
		cursor: pointer;
	}
	.save-btn:disabled {
		background: #a0aec0;
		cursor: not-allowed;
	}

	.save-hint {
		font-size: 13px;
		color: #666;
	}
	.save-hint.saved {
		color: var(--color-green);
	}
	.save-hint.error {
		color: var(--color-red);
	}

	:global(.node-card.flash) {
		box-shadow: 0 0 0 3px #ffd166;
	}
</style>
