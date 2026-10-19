import type { Attachment } from "svelte/attachments";
import { interactiveManager } from "./interactive-registry";

let idCounter = 0;

export function interactive(id?: string): Attachment<HTMLElement> {
	return (node) => {
		let regionId = id ?? `interactive-${idCounter++}`;
		interactiveManager.registerInteractiveElement(regionId, node);
		
		return () => {
			interactiveManager.unregisterInteractiveElement(regionId);
		};
	};
}