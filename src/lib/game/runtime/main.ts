// 코드흐름:
// ┌──────────────────── Parent Svelte App ──────────────────┐
// │                                                         │
// │  runtimeStore                                           │
// │       │                                                 │
// │       ▼                                                 │
// │  RuntimeBridge                                          │
// │       │                                                 │
// │       │ postMessage / MessageChannel                    │
// │       │                                                 │
// │  ┌────▼─────────────────────────────────────────────┐   │
// │  │                 Game iframe                      │   │
// │  │                                                   │   │
// │  │  Game Script                                      │   │
// │  │    └── Game API                                   │   │
// │  │         ├── getState()                            │   │
// │  │         ├── getLocation()                         │   │
// │  │         ├── emitEvent()                            │   │
// │  │         ├── showDialogue()                         │   │
// │  │         └── requestAction()                        │   │
// │  └───────────────────────────────────────────────────┘   │
// │                                                         │
// └───────────────────────────┬─────────────────────────────┘
//                             │
//                      Game Services
//                             │
//                          Tauri IPC
//                             │
//                        Rust Engine

// 보류(Deferred): 외부 HTML/JS 게임용 iframe SDK. 지금 플레이 화면은 Svelte가 직접 그린다.
// 빌드 입력(game-runtime.html)에서 빠져 있고, SDK를 다시 시작할 때 이 파일에서 이어간다.
import { waitForInit } from "./GameApiClient";

export async function run() {
	const init = await waitForInit();
	return init;
}
