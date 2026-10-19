// 이 파일은 호스트(신뢰하는 Svelte 앱) 쪽에서 실행됩니다.
// iframe이 보낸 GAME_API_CALL을 받아 실제 렌더링 콜백을 실행하고,
// 결과를 다시 iframe으로 돌려보냅니다.

import {
  isGameApiCall,
  isGameReady,
  type GameApiResponseMessage,
  type GameInitMessage,
} from './protocol';
import type { GameState, Scene, StateDefs, StateChangeInstruction } from '../types';

export interface GameHostHandlers {
  onTextboxShow: (args: { nodeId: string; speaker?: string; text: string }) => Promise<void>;
  onChoiceBoxShow: (args: {
    nodeId: string;
    choices: { text: string }[];
  }) => Promise<number>;
  onProgressReport: (args: { nodeId: string; state: GameState }) => Promise<void>;
  onStorageLoad: () => Promise<{ nodeId: string; state: GameState } | null>;
  onNpcSay: (args: {
    nodeId: string;
    characterId: string;
    userMessage: string;
    state: GameState;
  }) => Promise<{
    speakerName: string;
    lines: string[];
    stateChanges: StateChangeInstruction[];
    choices: string[];
  }>;
  /** iframe이 로드를 마치고 GAME_READY를 보냈을 때 호출됩니다. 이 시점에 sendInit()을 호출해주세요. */
  onReady: () => void;
}

export class GameHost {
  private iframe: HTMLIFrameElement;
  private handlers: GameHostHandlers;
  private listener: (event: MessageEvent) => void;

  constructor(iframe: HTMLIFrameElement, handlers: GameHostHandlers) {
    this.iframe = iframe;
    this.handlers = handlers;
    this.listener = (event) => {
      void this.handleMessage(event);
    };
    window.addEventListener('message', this.listener);
  }

  destroy(): void {
    window.removeEventListener('message', this.listener);
  }

  /** iframe이 GAME_READY를 보낸 뒤, DB에서 읽어온 씬 데이터를 넘겨줄 때 호출합니다. */
  sendInit(payload: {
    scene: Scene;
    stateDefs: StateDefs;
    resume?: { nodeId: string; state: GameState };
  }): void {
    const message: GameInitMessage = { type: 'GAME_INIT', ...payload };
    this.iframe.contentWindow?.postMessage(message, '*');
  }

  private async handleMessage(event: MessageEvent): Promise<void> {
    // allow-scripts만 준 sandbox iframe은 origin이 항상 "null"이라
    // origin 문자열 비교 대신, 메시지 출처가 우리가 만든 그 iframe인지
    // event.source로 확인합니다. 이게 여기서 진짜 신뢰 경계입니다.
    if (event.source !== this.iframe.contentWindow) return;

    if (isGameReady(event.data)) {
      this.handlers.onReady();
      return;
    }

    if (!isGameApiCall(event.data)) return;

    const { callId, method, args } = event.data;
    let result: unknown;

    switch (method) {
      case 'textbox.show':
        result = await this.handlers.onTextboxShow(
          args as { nodeId: string; speaker?: string; text: string }
        );
        break;
      case 'choiceBox.show':
        result = await this.handlers.onChoiceBoxShow(
          args as { nodeId: string; choices: { text: string }[] }
        );
        break;
      case 'progress.report':
        result = await this.handlers.onProgressReport(
          args as { nodeId: string; state: GameState }
        );
        break;
      case 'storage.load':
        result = await this.handlers.onStorageLoad();
        break;
      case 'npc.say':
        result = await this.handlers.onNpcSay(
          args as {
            nodeId: string;
            characterId: string;
            userMessage: string;
            state: GameState;
          }
        );
        break;
      default:
        console.warn(`[GameHost] unknown method: ${method}`);
        return;
    }

    const response: GameApiResponseMessage = {
      type: 'GAME_API_RESPONSE',
      callId,
      result,
    };
    this.iframe.contentWindow?.postMessage(response, '*');
  }
}
