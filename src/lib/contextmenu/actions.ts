import type { ContextMenuContext } from "./types";

export type ContextAction = (context?: ContextMenuContext, payload?: unknown) => void | Promise<void>;

class ContextActionRegistry {
	private actions = new Map<string, ContextAction>();

	register(id: string, action: ContextAction) {
		this.actions.set(id, action);

		return () => {
			this.actions.delete(id);
		};
	}

	/** 실제 컨텍스트 action을 실행하는 메서드
	 * @param {string} id - 해당 registry에 등록된 id action만 실행됨
	 * @param payload - 필요 시 넘겨 받을 event 데이터
	 */
	async execute(id: string, context?: ContextMenuContext, payload?: unknown) {
		const action = this.actions.get(id);

		if (!action) {
			console.warn(`Context action not found: ${id}`);
			return;
		}

		await action(context, payload);
	}
}

export const contextActionRegistry = new ContextActionRegistry();
