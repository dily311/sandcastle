<script lang="ts">
  import type { NodeDraft } from './draft';
  import { createChoiceDraft, createBranchDraft } from './draft';
  import type { ConditionOp } from '../types';
  import type { ValidationError } from './validateScene';
  import type { CharacterOption } from '../db/rowTypes';

  interface Props {
    node: NodeDraft;
    allNodes: NodeDraft[];
    variableKeys: string[];
    characterOptions: CharacterOption[];
    errors: ValidationError[];
    onDelete: () => void;
  }

  let { node, allNodes, variableKeys, characterOptions, errors, onDelete }: Props = $props();

  const conditionOps: { value: ConditionOp; label: string }[] = [
    { value: 'gte', label: '이상' },
    { value: 'lte', label: '이하' },
    { value: 'gt', label: '초과' },
    { value: 'lt', label: '미만' },
    { value: 'eq', label: '같음' },
    { value: 'has', label: '참(true)' },
  ];

  // node 자신은 next 옵션에서 빼서 자기 자신을 가리키는 실수를 막습니다.
  const nextOptions = $derived(allNodes.filter((n) => n.id !== node.id));

  function addChoice() {
    node.choices.push(createChoiceDraft());
  }
  function removeChoice(index: number) {
    node.choices.splice(index, 1);
  }
  function addBranch() {
    node.branches.push(createBranchDraft());
  }
  function removeBranch(index: number) {
    node.branches.splice(index, 1);
  }
</script>

<div class="node-card" id={`node-${node.id}`} class:has-error={errors.some((e) => e.severity === 'error')}>
  <div class="node-head">
    <select bind:value={node.type}>
      <option value="line">대사 (line)</option>
      <option value="choice">선택지 (choice)</option>
      <option value="branch">분기 (branch)</option>
      <option value="ai">AI 대사 (ai)</option>
    </select>
    <span class="node-key">{node.nodeKey}</span>
    <button class="delete-btn" onclick={onDelete} title="노드 삭제">✕</button>
  </div>

  {#if node.type === 'line'}
    <input class="text-input" placeholder="화자 (비워두면 나레이션)" bind:value={node.speaker} />
    <textarea class="text-area" placeholder="대사 내용" bind:value={node.text}></textarea>
    <label class="field-label">
      다음 노드
      <select bind:value={node.next}>
        <option value={null}>— 없음 (씬 종료) —</option>
        {#each nextOptions as n (n.id)}
          <option value={n.id}>{n.nodeKey}</option>
        {/each}
      </select>
    </label>
  {:else if node.type === 'choice'}
    <div class="list-section">
      {#each node.choices as choice, i (choice.id)}
        <div class="row-card">
          <input class="text-input" placeholder="선택지 텍스트" bind:value={choice.text} />
          <label class="field-label">
            다음 노드
            <select bind:value={choice.next}>
              <option value={null}>— 없음 (씬 종료) —</option>
              {#each nextOptions as n (n.id)}
                <option value={n.id}>{n.nodeKey}</option>
              {/each}
            </select>
          </label>
          <div class="effect-row">
            <span>효과:</span>
            <select bind:value={choice.effectVariable}>
              <option value="">(없음)</option>
              {#each variableKeys as key}
                <option value={key}>{key}</option>
              {/each}
            </select>
            {#if choice.effectVariable}
              <select bind:value={choice.effectOp}>
                <option value="add">증가/감소</option>
                <option value="set">값 설정</option>
              </select>
              <input class="value-input" placeholder="값" bind:value={choice.effectValue} />
            {/if}
          </div>
          <button class="remove-btn" onclick={() => removeChoice(i)}>선택지 삭제</button>
        </div>
      {/each}
      <button class="add-btn" onclick={addChoice}>+ 선택지 추가</button>
    </div>
  {:else if node.type === 'branch'}
    <div class="list-section">
      {#each node.branches as branch, i (branch.id)}
        <div class="row-card">
          <div class="condition-row">
            <span>조건:</span>
            <select bind:value={branch.conditionVariable}>
              <option value="">(없음 = 기본/fallback)</option>
              {#each variableKeys as key}
                <option value={key}>{key}</option>
              {/each}
            </select>
            {#if branch.conditionVariable}
              <select bind:value={branch.conditionOp}>
                {#each conditionOps as op}
                  <option value={op.value}>{op.label}</option>
                {/each}
              </select>
              <input class="value-input" placeholder="값" bind:value={branch.conditionValue} />
            {/if}
          </div>
          <label class="field-label">
            다음 노드
            <select bind:value={branch.next}>
              <option value={null}>— 없음 (씬 종료) —</option>
              {#each nextOptions as n (n.id)}
                <option value={n.id}>{n.nodeKey}</option>
              {/each}
            </select>
          </label>
          <button class="remove-btn" onclick={() => removeBranch(i)}>분기 삭제</button>
        </div>
      {/each}
      <button class="add-btn" onclick={addBranch}>+ 분기 추가</button>
    </div>
  {:else if node.type === 'ai'}
    <label class="field-label">
      캐릭터
      <select bind:value={node.characterId}>
        <option value="">— 선택 안 함 —</option>
        {#each characterOptions as c (c.id)}
          <option value={c.id}>{c.name}</option>
        {/each}
      </select>
    </label>

    <label class="field-label">
      시작 상황 설명 (AI가 이 상황에서부터 대화를 이어갑니다)
      <textarea
        class="text-area"
        placeholder="예: 방금 선물을 받은 상황이야"
        bind:value={node.promptContext}
      ></textarea>
    </label>

    <label class="field-label">
      최대 왕복 횟수 (AI가 계속 선택지를 주면서 이어갈 수 있는 최대 횟수, 비워두면 기본값 5)
      <input
        class="text-input"
        type="number"
        min="1"
        placeholder="예: 5"
        value={node.maxTurns ?? ''}
        oninput={(e) => {
          const v = (e.target as HTMLInputElement).value;
          node.maxTurns = v === '' ? null : Number(v);
        }}
      />
    </label>

    <label class="field-label">
      다음 노드
      <select bind:value={node.next}>
        <option value={null}>— 없음 (씬 종료) —</option>
        {#each nextOptions as n (n.id)}
          <option value={n.id}>{n.nodeKey}</option>
        {/each}
      </select>
    </label>
  {/if}

  {#each errors as err}
    <p class="error-badge {err.severity}">{err.message}</p>
  {/each}
</div>

<style>
  .node-card {
    border: 1px solid rgba(0, 0, 0, 0.15);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 12px;
    background: var(--card-bg);
  }

  .node-card.has-error {
    border-color: var(--color-red);
    border-width: 2px;
  }

  .node-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }

  .node-key {
    font-family: monospace;
    font-size: 12px;
    color: #888;
    flex: 1;
  }

  .delete-btn {
    background: none;
    border: none;
    color: var(--color-red);
    cursor: pointer;
    font-size: 14px;
  }

  .text-input,
  .text-area,
  .value-input,
  select {
    display: block;
    width: 100%;
    box-sizing: border-box;
    padding: 6px 8px;
    margin-bottom: 6px;
    border: 1px solid rgba(0, 0, 0, 0.2);
    border-radius: 4px;
    font: inherit;
  }

  .text-area {
    min-height: 60px;
    resize: vertical;
  }

  .field-label {
    display: block;
    font-size: 12px;
    color: #666;
    margin-bottom: 6px;
  }

  .list-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .row-card {
    border: 1px dashed rgba(0, 0, 0, 0.2);
    border-radius: 6px;
    padding: 8px;
  }

  .effect-row,
  .condition-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 6px;
    flex-wrap: wrap;
  }

  .effect-row select,
  .condition-row select,
  .value-input {
    width: auto;
    margin-bottom: 0;
  }

  .add-btn {
    align-self: flex-start;
    /* background: #edf2f7; */
    border: 1px solid rgba(0, 0, 0, 0.15);
    border-radius: 6px;
    padding: 6px 10px;
    cursor: pointer;
  }

  .remove-btn {
    background: none;
    border: none;
    color: var(--color-red);
    font-size: 12px;
    cursor: pointer;
    padding: 0;
  }

  .error-badge {
    margin: 4px 0 0;
    font-size: 12px;
  }

  .error-badge.error {
    color: var(--color-red);
  }

  .error-badge.warning {
    color: var(--color-orange);
  }
</style>
