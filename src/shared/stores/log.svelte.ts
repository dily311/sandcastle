// 이번 실행 동안의 오류·알림 기록 (설정 → 로그 화면). 앱을 끄면 사라짐.
export type LogLevel = "error" | "warn" | "info";

export interface LogEntry {
	id: number;
	level: LogLevel;
	time: number;
	/** 어디서: chat, import, task, app … */
	source: string;
	message: string;
	details?: unknown;
}

const MAX = 500;
let seq = 0;

class LogStore {
	entries = $state<LogEntry[]>([]);

	add(level: LogLevel, source: string, message: string, details?: unknown) {
		this.entries = [{ id: ++seq, level, time: Date.now(), source, message, details }, ...this.entries].slice(0, MAX);
	}
	error(source: string, message: string, details?: unknown) {
		this.add("error", source, message, details);
	}
	warn(source: string, message: string, details?: unknown) {
		this.add("warn", source, message, details);
	}
	info(source: string, message: string, details?: unknown) {
		this.add("info", source, message, details);
	}
	clear() {
		this.entries = [];
	}
	get errorCount() {
		return this.entries.filter((e) => e.level === "error").length;
	}
}

export const logs = new LogStore();

/** 잡히지 않은 오류도 로그로 */
export function captureGlobalErrors() {
	window.addEventListener("error", (e) => logs.error("app", e.message, { file: e.filename, line: e.lineno }));
	window.addEventListener("unhandledrejection", (e) => {
		const r = e.reason as { message?: string } | undefined;
		logs.error("app", r?.message ?? String(e.reason), e.reason);
	});
}
