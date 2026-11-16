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

/** 잡히지 않은 오류도 로그로 (화면 오류 + 백엔드 panic, 1-D-6). 밖으로 보내지 않는다 */
export function captureGlobalErrors() {
	// 백엔드 Rust panic (sys/panic_log.rs가 파일에도 남김)
	void import("$shared/ipc/events").then(({ onEvent }) =>
		onEvent<{ message: string; location: string | null; thread: string }>("app-panic", (p) => logs.error("panic", p.message, { location: p.location, thread: p.thread })),
	);
	window.addEventListener("error", (e) => {
		// 브라우저가 크기 감시(ResizeObserver) 알림을 한 프레임 미룰 때 내는 알림. 오류가 아니고 화면에 영향이 없다
		if (/ResizeObserver loop/i.test(e.message ?? "")) return;
		logs.error("app", e.message, { file: e.filename, line: e.lineno });
	});
	window.addEventListener("unhandledrejection", (e) => {
		const r = e.reason as { message?: string } | undefined;
		logs.error("app", r?.message ?? String(e.reason), e.reason);
	});
}
