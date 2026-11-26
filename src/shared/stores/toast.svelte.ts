import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { logs, type LogLevel } from "./log.svelte";

export type ToastTone = "info" | "success" | "warning" | "error";

export interface ToastProgress {
	current?: number;
	total?: number;
	indeterminate?: boolean;
}

export interface Toast {
	id: number;
	tone: ToastTone;
	message: string;
	/** 자세히 (오류 코드·안내 등) */
	detail?: string;
	/** 진행률 막대 (동기화·데이터 정리 등 백그라운드 작업) */
	progress?: ToastProgress;
	/** 토스트 본문을 누르면 바로 닫기 */
	dismissOnClick?: boolean;
	/** 사용자가 눌러서 닫았을 때 부르는 콜백 */
	onDismiss?: () => void;
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
		if (t.log !== false) logs.add(LOG_LEVEL[t.tone], t.source ?? get(_)("toast.defaultSource"), t.message, t.detail);
		const id = ++seq;
		this.items = [...this.items, { ...t, id }].slice(-4);
		if (ms > 0) setTimeout(() => this.dismiss(id, false), ms);
		return id;
	}

	/** 이미 떠 있는 토스트의 문구·진행률을 제자리에서 갱신한다 (이미 닫혔으면 false) */
	update(id: number, patch: Partial<Omit<Toast, "id">>): boolean {
		const idx = this.items.findIndex((t) => t.id === id);
		if (idx < 0) return false;
		const next = [...this.items];
		next[idx] = { ...next[idx], ...patch };
		this.items = next;
		return true;
	}

	dismiss(id: number, byUser = false) {
		const target = this.items.find((t) => t.id === id);
		this.items = this.items.filter((t) => t.id !== id);
		if (byUser) target?.onDismiss?.();
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
