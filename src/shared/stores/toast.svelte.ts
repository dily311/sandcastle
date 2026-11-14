export type ToastTone = "info" | "success" | "warning" | "error";

export interface Toast {
	id: number;
	tone: ToastTone;
	message: string;
	/** 자세히 (오류 코드 등) */
	detail?: string;
	action?: { label: string; run: () => void };
}

let seq = 0;

class ToastStore {
	items = $state<Toast[]>([]);

	show(t: Omit<Toast, "id">, ms = t.tone === "error" ? 6000 : 3500) {
		const id = ++seq;
		this.items = [...this.items, { ...t, id }].slice(-4);
		if (ms > 0) setTimeout(() => this.dismiss(id), ms);
		return id;
	}
	dismiss(id: number) {
		this.items = this.items.filter((t) => t.id !== id);
	}
	info(message: string) {
		return this.show({ tone: "info", message });
	}
	success(message: string) {
		return this.show({ tone: "success", message });
	}
	error(message: string, detail?: string) {
		return this.show({ tone: "error", message, detail });
	}
}

export const toasts = new ToastStore();
