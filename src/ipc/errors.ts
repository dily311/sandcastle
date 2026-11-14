import { toasts } from "$/lib/stores/toast.svelte";
export interface AppErrorResponse {
  code: string;
  category: string;
  message: string;
  retryable: boolean;
  details?: Record<string, unknown>;
}

function isAppError(e: unknown): e is AppErrorResponse {
	if (typeof e !== "object" || e === null || !("kind" in e) || !("message" in e)) {
		return false;
	}

	const error = e as Record<string, unknown>;

	return (
		typeof error.kind === "string" &&
		typeof error.message === "string"
	);
}
function parseErrorMessage(e: string): AppErrorResponse {
	try {
		const parsed = JSON.parse(e);
		if (isAppError(parsed)) return parsed;
	} catch (error) {
		// JSON이 아니면 그냥 문자열 에러
	}
	return { code: "Unknown", category: "Io", message: e, retryable: false };
}

export function normalizeError(e: unknown): AppErrorResponse {
	if (isAppError(e)) return e;
	if (e instanceof Error) {
		return parseErrorMessage(e.message);
	}

	// rust에서 문자열로 넘어오는 경우
	if (typeof e === "string") {
		return parseErrorMessage(e);
	}
	return { code: "Unknown", category: "Io", message: "알 수 없는 오류가 발생했습니다.", retryable: false };
}

export function showError(e: unknown): void {
	const err = normalizeError(e);
	toasts.showToast({type: "error", message: `${err.code}: ${err.message}`});
}