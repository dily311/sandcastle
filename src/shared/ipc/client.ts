// 백엔드 호출 진입점. 화면은 bindings의 commands를 직접 쓰지 않고 call()로 감싸서 쓴다.
//   const list = await call(commands.listPromptPresets(), { source: "preset" });
// - 실패하면 AppError로 바꿔서 던지고, 로그에 남기고, 기본으로 토스트를 띄움
import type { AppErrorResponse_Serialize, ErrorCode } from "$ipc/bindings";
import { logs } from "$shared/stores/log.svelte";
import { toasts } from "$shared/stores/toast.svelte";

export { commands } from "$ipc/bindings";

export class AppError extends Error {
	readonly code: ErrorCode | "UNKNOWN";
	readonly category: string;
	readonly retryable: boolean;
	readonly details?: unknown;

	constructor(r: { code: ErrorCode | "UNKNOWN"; category: string; message: string; retryable: boolean; details?: unknown }) {
		super(r.message);
		this.name = "AppError";
		this.code = r.code;
		this.category = r.category;
		this.retryable = r.retryable;
		this.details = r.details;
	}
}

function isResponse(e: unknown): e is AppErrorResponse_Serialize {
	return typeof e === "object" && e !== null && "code" in e && "message" in e && "category" in e;
}

/** Tauri가 돌려준 무엇이든 AppError로 */
export function toAppError(e: unknown): AppError {
	if (e instanceof AppError) return e;
	if (isResponse(e)) return new AppError(e);
	if (typeof e === "string") {
		// 커맨드 이름이 틀렸거나 인자 타입이 다른 등 Tauri 자체 오류는 문자열로 온다
		try {
			const parsed = JSON.parse(e);
			if (isResponse(parsed)) return new AppError(parsed);
		} catch {
			/* 문자열 그대로 */
		}
		return new AppError({ code: "UNKNOWN", category: "INTERNAL", message: e, retryable: false });
	}
	if (e instanceof Error) return new AppError({ code: "UNKNOWN", category: "INTERNAL", message: e.message, retryable: false });
	return new AppError({ code: "UNKNOWN", category: "INTERNAL", message: "알 수 없는 오류", retryable: false });
}

export interface CallOptions {
	/** 로그에 남길 출처 */
	source?: string;
	/** 토스트 없이 조용히 (화면이 직접 오류를 표시할 때) */
	silent?: boolean;
}

export async function call<T>(p: Promise<T>, opts: CallOptions = {}): Promise<T> {
	try {
		return await p;
	} catch (e) {
		const err = toAppError(e);
		logs.error(opts.source ?? "ipc", `${err.code} · ${err.message}`, err.details);
		if (!opts.silent) toasts.error(err.message, err.code);
		throw err;
	}
}
