// iframe이 로드되면 이 스크립트가 실행됩니다.
// 1단계와 다른 점: 씬 데이터를 하드코딩 import하는 대신 호스트가
// DB에서 읽어와 보내주는 GAME_INIT 메시지를 기다립니다 (waitForInit).

import { Game, waitForInit } from './gameApiClient';
import { DialoguePlayer } from './DialoguePlayer';
import type { AiNode } from '../types';

const DEFAULT_MAX_TURNS = 5;

/**
 * AI 노드 하나를 처리합니다. AI가 <choice>를 계속 주는 한 왕복을 반복하고
 * (최대 maxTurns번), <choice> 없이 끝내면 리턴해서 node.next로 넘어갑니다.
 */
async function runAiNode(node: AiNode, player: DialoguePlayer): Promise<void> {
  const maxTurns = node.maxTurns ?? DEFAULT_MAX_TURNS;
  let userMessage = node.promptContext ?? '(상황에 맞게 대화를 시작해줘)';

  for (let turn = 0; turn < maxTurns; turn++) {
    const res = await Game.npc.say({
      nodeId: node.id,
      characterId: node.characterId,
      userMessage,
      state: player.state,
    });

    for (const line of res.lines) {
      // 클릭할 때까지 이 await가 resolve되지 않습니다 (한 줄씩 순서대로 보여줍니다).
      await Game.textbox.show({ nodeId: node.id, speaker: res.speakerName, text: line });
    }

    player.applyStateChanges(res.stateChanges);

    // 선택지가 뜨는 동안 플레이어가 저장 버튼을 눌러도 방금 반영된 상태가
    // 캐시돼있어야 하니, 여기서 한 번 더 보고합니다 (아래 바깥 루프의
    // progress.report는 이 AI 노드가 완전히 끝난 뒤에야 호출되거든요).
    await Game.progress.report({ nodeId: node.id, state: player.state });

    if (res.choices.length === 0) {
      return; // AI가 대화를 마쳤다는 뜻 -> node.next로 진행
    }

    if (turn === maxTurns - 1) {
      // 안전장치: 선택지가 더 있어도 여기서 강제로 끝냅니다.
      return;
    }

    const index = await Game.choiceBox.show({
      nodeId: node.id,
      choices: res.choices.map((text) => ({ text })),
    });
    userMessage = res.choices[index] ?? res.choices[0];
  }
}

async function run() {
  const init = await waitForInit();
  const player = new DialoguePlayer(init.scene, init.stateDefs);

  // 이전에 플레이하다 저장된 세이브가 있으면 거기서부터 이어서 시작합니다.
  const save = await Game.storage.load();
  if (save) {
    player.restore(save.nodeId, save.state);
  }

  while (true) {
    const node = player.current();

    if (!node) {
      await Game.textbox.show({ nodeId: 'end', text: '— 씬 종료 —' });
      break;
    }

    if (node.type === 'line') {
      // 클릭할 때까지 이 await가 resolve되지 않습니다 (호스트가 응답을 보낼 때까지 대기).
      await Game.textbox.show({ nodeId: node.id, speaker: node.speaker, text: node.text });
      player.advance();
    } else if (node.type === 'ai') {
      await runAiNode(node, player);
      player.advance();
    } else {
      const index = await Game.choiceBox.show({
        nodeId: node.id,
        choices: node.choices.map((c) => ({ text: c.text })),
      });
      player.choose(index);
    }

    // 노드를 넘어갈 때마다 호스트에 현재 위치를 알려줍니다.
    // 이건 DB에 저장하는 게 아니라 "저장 버튼을 누르면 여기가 저장될 지점"을
    // 호스트가 알고 있게 하는 것뿐입니다. 실제 저장은 플레이어가 버튼을 눌러야 일어나요.
    const currentId = player.getCurrentId();
    if (currentId) {
      await Game.progress.report({ nodeId: currentId, state: player.state });
    }
  }
}

run();
