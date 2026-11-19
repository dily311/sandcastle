import { logs, type LogLevel } from "./log.svelte";

export type ToastTone = "info" | "success" | "warning" | "error";

export interface Toast {
	id: number;
	tone: ToastTone;
	message: string;
	/** 자세히 (오류 코드 등) */
	detail?: string;
	action?: { label: string; run: () => void };
	/** 설정 > 로그에 남길 때의 출처 (기본 "알림") */
	source?: string;
	/** false면 로그에 남기지 않음 (이미 로그에 남긴 오류, 진행 표시처럼 같은 말이 되풀이되는 알림) */
	log?: boolean;
}

const LOG_LEVEL: Record<ToastTone, LogLevel> = { error: "error", warning: "warn", info: "info", success: "info" };

let seq = 0;

class ToastStore {
	items = $state<Toast[]>([]);

	show(t: Omit<Toast, "id">, ms = t.tone === "error" ? 6000 : 3500) {
		// 화면에 뜨는 알림은 모두 설정 > 로그에도 남는다 (지나가 버려도 나중에 볼 수 있게)
		if (t.log !== false) logs.add(LOG_LEVEL[t.tone], t.source ?? "알림", t.message, t.detail);
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
