<script lang="ts">
  import GameTitleScreen from './GameTitleScreen.svelte';
  import GameStage from './GameStage.svelte';
  import type { NpcResponder } from './npc/npcResponder';
	import GameEditorPage from './edior/GameEditorPage.svelte';

  interface Props {
    gameId: string;
    gameTitle?: string;
    startSceneKey: string;
    npcResponder: NpcResponder;
    maxSlots?: number;
  }

  let { gameId, gameTitle, startSceneKey, npcResponder, maxSlots = 3 }: Props = $props();

  let mode = $state<'title' | 'playing'>('title');
	let showDebug = $state(false);
  let activeSlot = $state<number | null>(null);

  function handlePlay(slotNumber: number) {
    activeSlot = slotNumber;
    mode = 'playing';
  }

  function handleExitToTitle() {
    mode = 'title';
    activeSlot = null;
  }
</script>

{#if mode === 'title'}
  <GameTitleScreen {gameId} {gameTitle} {maxSlots} onPlay={handlePlay} />
{:else if activeSlot !== null}
  <GameStage
    {gameId}
    {startSceneKey}
    {npcResponder}
    slotNumber={activeSlot}
    onExit={handleExitToTitle}
    bind:showDebug={showDebug}
  />
{/if}

{#if showDebug}
  <GameEditorPage {gameId} sceneKey={startSceneKey} />
{/if}