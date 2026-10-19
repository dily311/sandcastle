// toast.ts
export type ToastType = "success" | "error" | "info";

export interface Toast {
	id: number;
	type: ToastType;
	message: string;
}

class ToastStore {
	toasts = $state<Toast[]>([]);
	nextId = $state(0);

	get toastsValue() {
		return this.toasts;
	}

	showToast(toast: Omit<Toast, "id">, duration = 3000): void {
		const id = this.nextId++;
		this.toasts = [...this.toasts, { ...toast, id }];
		setTimeout(() => {
			this.toasts = this.toasts.filter((t) => t.id !== id);
		}, duration);
	}
}

export const toasts = new ToastStore();
