import type { ContextMenuItem, ContextMenuSource } from "./types";

class ContextMenuRegistry {
	private sources: ContextMenuSource[] = [];

	register(source: ContextMenuSource) {
		this.sources.push(source);

		return () => {
			this.unregister(source.featureId);
		};
	}

	unregister(featureId: string) {
		this.sources = this.sources.filter((source) => source.featureId !== featureId);
	}

	getItems(): ContextMenuItem[] {
		const result: ContextMenuItem[] = [];

		const sorted = [...this.sources].sort((a, b) => (a.priority ?? 0) - (b.priority ?? 0));

		for (const source of sorted) {
			for (const item of source.items) {
				result.push(item);
			}
		}

		return result;
	}

	getFeatureItems(featureId: string) {
		const source = this.sources.find((source) => source.featureId === featureId);

		if (!source) {
			return [];
		}

		return source.items;
	}
}

export const contextMenuRegistry = new ContextMenuRegistry();
