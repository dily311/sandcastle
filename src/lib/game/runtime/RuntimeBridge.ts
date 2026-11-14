import type { ContinueGameResponse, GameCommand, RenderState } from "$features/game/types";
import type { HostMessage } from "../host/protocol";

class createRuntimeBridge {
	async initialize(value: ContinueGameResponse) {}

	postMessage(value: HostMessage): HostMessage {
		return value;
	}
}

export const runtimeBridge = new createRuntimeBridge();
