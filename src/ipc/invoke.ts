import { invoke } from "@tauri-apps/api/core";
import { normalizeError } from "./errors";
import { listen, type EventCallback, type Options, type UnlistenFn } from "@tauri-apps/api/event";
import { type CommandName, type CommandRegistry, type EventName, type EventRegistry } from "./commandTypes";
import { mockHandlers } from "./mockup";
import { toasts } from "$/lib/stores/toast.svelte";

export async function invokeCommand<K extends CommandName>(
	command: K,
	args?: CommandRegistry[K]["args"],
): Promise<CommandRegistry[K]["response"]> {
	try {
		// 모킹 모드의 경우
		if (!__IS_TAURI__) {
			const handler = mockHandlers[command];
			if (handler) {
				console.log(`[Tauri Mock] ${command} called with ${args}`);
				return await handler(args);
			}
			return null;
		}

		// 실제 통신 결과
		console.info("[Tauri ipc] ", command, args);
		const result = await invoke<CommandRegistry[K]["response"]>(command, args);
		console.info("[Tauri ipc result] ", command, result);
		return result;
	} catch (e) {
		toasts.showToast({type: "error", message: JSON.stringify(e)});
		throw normalizeError(e);
	}
}

export async function listenCommand<K extends EventName>(
	event: K,
	handler: EventCallback<EventRegistry[K]["response"]>,
	options?: Options,
): Promise<UnlistenFn> {
	try {
		// 모킹 모드의 경우
		if (!__IS_TAURI__) {
			// 실행이 안되겠지
			return () => {
				return;
			};
		}

		const result = await listen<EventRegistry[K]["response"]>(event, handler, options);
		return result;
	} catch (e) {
		throw normalizeError(e);
	}
}
