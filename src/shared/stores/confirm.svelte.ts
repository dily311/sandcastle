// 확인 창: const ok = await confirmAction({ title: "대화 삭제", message: "...", danger: true })
// 앱 셸의 <ConfirmHost />가 그린다.
export interface ConfirmRequest {
	title: string;
	message?: string;
	confirmLabel?: string;
	danger?: boolean;
	/** 값을 입력받을 때 (이름 등). 있으면 입력값을 돌려준다 */
	input?: { value: string; placeholder?: string };
}

interface Pending extends ConfirmRequest {
	resolve: (v: string | boolean | null) => void;
}

class ConfirmStore {
	current = $state<Pending | null>(null);

	ask(req: ConfirmRequest): Promise<string | boolean | null> {
		this.current?.resolve(null);
		return new Promise((resolve) => (this.current = { ...req, resolve }));
	}
	close(v: string | boolean | null) {
		const c = this.current;
		this.current = null;
		c?.resolve(v);
	}
}

export const confirmStore = new ConfirmStore();

export async function confirmAction(req: Omit<ConfirmRequest, "input">): Promise<boolean> {
	return (await confirmStore.ask(req)) === true;
}

export async function promptText(req: ConfirmRequest & { input: { value: string; placeholder?: string } }): Promise<string | null> {
	const v = await confirmStore.ask(req);
	return typeof v === "string" ? v : null;
}
