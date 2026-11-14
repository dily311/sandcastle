import type { ContinueGameResponse, GameCommand, RenderState } from "$/lib/type/game";
import type { HostMessage } from "../host/protocol";

class createRuntimeBridge {
	async initialize(value: ContinueGameResponse) {}

	postMessage(value: HostMessage): HostMessage {
		return value;
	}
}

export const runtimeBridge = new createRuntimeBridge();
