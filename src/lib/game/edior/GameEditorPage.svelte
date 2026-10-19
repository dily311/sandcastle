<script lang="ts">
  import { onMount } from 'svelte';
  import { gameCommands } from '$ipc/commands';
  import { gameRowToInput, stateDefToInput, sceneRowToInputUnchanged } from '../db/converters';
  import { sceneRowToDraft, sceneDraftToInput, type SceneDraft } from './draft';
  import SceneEditor from './SceneEditor.svelte';
  import type { GameBundleRow, GameBundleInput, CharacterOption } from '../db/rowTypes';

  interface Props {
    gameId: string;
    sceneKey: string;
  }

  let { gameId, sceneKey }: Props = $props();

  let bundle = $state<GameBundleRow | null>(null);
  let sceneDraft = $state<SceneDraft | null>(null);
  let characterOptions = $state<CharacterOption[]>([]);
  let loadError = $state<string | null>(null);

  const variableKeys = $derived(bundle ? bundle.stateDefs.map((d) => d.key) : []);

  onMount(async () => {
    try {
      const [loaded, characters] = await Promise.all([
        gameCommands.loadGameBundle({ gameId }),
        gameCommands.listCharacters(),
      ]);

      const sceneRow = loaded.scenes.find((s) => s.sceneKey === sceneKey);
      if (!sceneRow) {
        loadError = `씬을 찾을 수 없습니다: ${sceneKey}`;
        return;
      }
      bundle = loaded;
      sceneDraft = sceneRowToDraft(sceneRow);
      characterOptions = characters;
    } catch (err) {
      console.error(err);
      loadError = String(err);
    }
  });

  async function handleSave() {
    if (!bundle || !sceneDraft) return;

    const input: GameBundleInput = {
      game: gameRowToInput(bundle.game),
      stateDefs: bundle.stateDefs.map(stateDefToInput),
      scenes: bundle.scenes.map((s) =>
        s.sceneKey === sceneKey ? sceneDraftToInput(sceneDraft!) : sceneRowToInputUnchanged(s)
      ),
    };

    const updated = await gameCommands.saveGameBundle({ input });

    // 서버가 돌려준 최신 상태로 교체합니다. (id가 새로 채번됐거나,
    // sort_order가 DB 기준으로 재정렬됐거나 하는 경우까지 화면에 정확히 반영하기 위해서예요.)
    bundle = updated;
    const refreshedScene = updated.scenes.find((s) => s.sceneKey === sceneKey);
    if (refreshedScene) {
      sceneDraft = sceneRowToDraft(refreshedScene);
    } else {
      // 정상이라면 못 찾을 일이 없지만, 혹시라도 sceneKey가 저장 과정에서
      // 바뀌었다면 편집 상태를 잃지 않도록 기존 draft를 그대로 둡니다.
      console.warn(`[GameEditorPage] 저장 후 씬을 다시 찾지 못했습니다: ${sceneKey}`);
    }
  }
</script>

{#if loadError}
  <p class="load-error">게임 데이터를 불러오지 못했어요: {loadError}</p>
{:else if !bundle || !sceneDraft}
  <p>불러오는 중...</p>
{:else}
  <SceneEditor scene={sceneDraft} {variableKeys} {characterOptions} onSave={handleSave} />
{/if}


<style>
	.load-error {
		color: var(--color-red);
	}
</style>
