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

import { waitForInit } from "./GameApiClient";

async function run() {
	const inti = await waitForInit();

	while (true) {}
}

run();
