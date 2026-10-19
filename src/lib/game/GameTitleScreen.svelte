<script lang="ts">
  import { onMount } from 'svelte';
  import { gameCommands } from '$ipc/commands';
	import type { GamePlayerSessions } from '../types/game';

  interface Props {
    gameId: string;
    gameTitle?: string;
    onPlay: (slotNumber: number) => void;
  }

  let { gameId, gameTitle = '', onPlay }: Props = $props();

  let saves = $state<GamePlayerSessions[]>([]);
  let loading = $state(true);
  let confirmingSlot = $state<string | null>(null);
  let busySlot = $state<string | null>(null);

  async function refresh() {
    loading = true;
    try {
      saves = await gameCommands.listSaves({ gameId });
    } catch (err) {
      console.error(err);
    } finally {
      loading = false;
    }
  }

  onMount(refresh);

  function formatDate(ts: number): string {
    return new Date(ts).toLocaleString();
  }

  function handleContinue(sessionId: string) {
    onPlay(sessionId);
  }

  async function handleNewGameClick(sessionId: string, hasSave: boolean) {
    if (!hasSave) {
      const input = {
        slotNumber: sessionId,
        stateSnapshot: {},
      };
      await gameCommands.saveGame({ gameId, input });
      onPlay(sessionId);
      return;
    }
    confirmingSlot = sessionId;
  }

  async function confirmNewGame(sessionId: string) {
    busySlot = sessionId;
    try {
      await gameCommands.deleteSave({ gameId, slotNumber: sessionId });
      confirmingSlot = null;
      onPlay(sessionId);
    } catch (err) {
      console.error(err);
    } finally {
      busySlot = null;
    }
  }

  async function handleDelete(sessionId: string) {
    busySlot = sessionId;
    try {
      await gameCommands.deleteSave({ gameId, slotNumber: sessionId });

      await refresh();
    } catch (err) {
      console.error(err);
    } finally {
      busySlot = null;
    }
  }
</script>

<div class="title-screen">
  {#if gameTitle}
    <h1 class="game-title">{gameTitle}</h1>
  {/if}

  {#if loading}
    <p class="loading">불러오는 중...</p>
  {:else}
    <div class="slot-list">
      {#each saves as save}
        <div class="slot-card">
          <div class="slot-info">
            <span class="slot-label">슬롯 {save.id}</span>
            {#if save}
              <span class="slot-detail">마지막 저장: {formatDate(save.updatedAt)}</span>
            {:else}
              <span class="slot-detail empty">비어있음</span>
            {/if}
          </div>

          {#if confirmingSlot === save.id}
            <div class="confirm-row">
              <span>기존 진행 상황을 덮어써요. 계속할까요?</span>
              <button
                class="btn danger"
                disabled={busySlot === save.id}
                onclick={() => confirmNewGame(save.id)}
              >
                덮어쓰고 시작
              </button>
              <button class="btn ghost" onclick={() => (confirmingSlot = null)}>취소</button>
            </div>
          {:else}
            <div class="slot-actions">
              {#if save}
                <button class="btn primary" onclick={() => handleContinue(save.id)}>
                  이어하기
                </button>
              {/if}
              <button
                class="btn"
                disabled={busySlot === save.id}
                onclick={() => handleNewGameClick(save.id, !!save)}
              >
                새로 시작
              </button>
              {#if save}
                <button
                  class="btn ghost danger-text"
                  disabled={busySlot === save.id}
                  onclick={() => handleDelete(save.id)}
                >
                  삭제
                </button>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .title-screen {
    width: 960px;
    max-width: 100%;
    margin: 0 auto;
    padding: 32px 16px;
  }

  .game-title {
    text-align: center;
    margin-bottom: 24px;
    font-size: 28px;
  }

  .loading {
    text-align: center;
    color: rgba(255, 255, 255, 0.6);
  }

  .slot-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .slot-card {
    background: var(--card-bg);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 10px;
    padding: 14px 18px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
  }

  .slot-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .slot-label {
    font-weight: 600;
  }

  .slot-detail {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
  }

  .slot-detail.empty {
    font-style: italic;
  }

  .slot-actions,
  .confirm-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .confirm-row span {
    font-size: 13px;
    color: var(--color-yellow);
  }

  .btn {
    padding: 8px 14px;
    border-radius: 6px;
    /* border: 1px solid rgba(255, 255, 255, 0.2);
    background: rgba(255, 255, 255, 0.08); */
    color: inherit;
    cursor: pointer;
    font-size: 13px;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn.primary {
    background: var(--color-green);
    border-color: transparent;
    font-weight: 600;
  }

  .btn.ghost {
    background: transparent;
  }

  .btn.danger {
    background: var(--color-red);
    border-color: transparent;
  }

  .btn.danger-text {
    color: var(--color-red);
  }
</style>
