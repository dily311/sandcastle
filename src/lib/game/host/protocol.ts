// iframe(게임 코드) <-> host(Svelte 앱) 사이를 오가는 메시지 스키마입니다.
// 양쪽 모두 이 타입을 import해서 씁니다.

import type { Scene, StateDefs, GameState } from '../types';

export interface GameApiCallMessage {
  type: 'GAME_API_CALL';
  callId: string;
  method: string; // 예: 'textbox.show', 'choiceBox.show'
  args: unknown;
}

export interface GameApiResponseMessage {
  type: 'GAME_API_RESPONSE';
  callId: string;
  result: unknown;
}

export function isGameApiCall(data: unknown): data is GameApiCallMessage {
  return (
    typeof data === 'object' &&
    data !== null &&
    (data as { type?: unknown }).type === 'GAME_API_CALL'
  );
}

export function isGameApiResponse(data: unknown): data is GameApiResponseMessage {
  return (
    typeof data === 'object' &&
    data !== null &&
    (data as { type?: unknown }).type === 'GAME_API_RESPONSE'
  );
}

// 아래 두 메시지는 iframe이 로드된 뒤 실제 씬 데이터를 넘겨받기 위한 핸드셰이크입니다.
// GAME_API_CALL/RESPONSE와 분리한 이유: 이건 게임 로직의 "호출"이 아니라
// 게임이 시작되기 전에 한 번만 오가는 초기화 단계라서 의미가 다릅니다.

export interface GameReadyMessage {
  type: 'GAME_READY';
}

export interface GameInitMessage {
  type: 'GAME_INIT';
  scene: Scene;
  stateDefs: StateDefs;
  resume?: { nodeId: string; state: GameState };
}

export function isGameReady(data: unknown): data is GameReadyMessage {
  return (
    typeof data === 'object' &&
    data !== null &&
    (data as { type?: unknown }).type === 'GAME_READY'
  );
}

export function isGameInit(data: unknown): data is GameInitMessage {
  return (
    typeof data === 'object' &&
    data !== null &&
    (data as { type?: unknown }).type === 'GAME_INIT'
  );
}

// iframe이 로드되자마자 보내는 "나 준비됐어" 신호.
// 호스트는 이걸 받은 뒤에야 실제 씬 데이터를 보냅니다 (DB에서 불러오는 데
// 시간이 걸릴 수 있으니, iframe이 먼저 기다리는 구조가 안전합니다).
export interface GameReadyMessage {
  type: 'GAME_READY';
}

// 호스트가 DB에서 조립한 씬 데이터를 iframe에 넘겨줄 때 씁니다.
export interface GameInitMessage {
  type: 'GAME_INIT';
  scene: unknown; // types.ts의 Scene (protocol.ts는 도메인 타입에 의존하지 않게 unknown으로 둡니다)
  stateDefs: unknown; // types.ts의 StateDefs
  initialState?: unknown; // 저장된 세이브가 있으면 GameState, 없으면 undefined
}

