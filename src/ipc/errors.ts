import { toasts } from "$/lib/stores/toast.svelte";

export type AppErrorKind = "Validation" | "NotFound" | "Database" | "Archive" | "AiProvider" | "Window" | "Io";
export type AppError = {
	kind: AppErrorKind;
	message: string;
};
// TODO: 이걸 어떻게 파싱하지?
// API 연결 에러
type APIErrorName =
	| "API_KEY_MISSING"
	| "API_REQUEST"
	| "API_RESPONSE"
	| "TIME_OUT"
	| "NETWORK"
	| "PROJECT_LIMIT_REACHED"
	| "404_ERROR"
	| "503_ERROR";

const APP_ERROR_KINDS = ["Validation", "NotFound", "Database", "Archive", "AiProvider", "Window", "Io"] as const;
function isAppError(e: unknown): e is AppError {
	if (typeof e !== "object" || e === null || !("kind" in e) || !("message" in e)) {
		return false;
	}

	const error = e as Record<string, unknown>;

	return (
		typeof error.kind === "string" &&
		APP_ERROR_KINDS.includes(error.kind as AppErrorKind) &&
		typeof error.message === "string"
	);
}
function parseErrorMessage(e: string): AppError {
	try {
		const parsed = JSON.parse(e);
		if (isAppError(parsed)) return parsed;
	} catch (error) {
		// JSON이 아니면 그냥 문자열 에러
	}
	return { kind: "Io", message: e };
}

export function normalizeError(e: unknown): AppError {
	if (isAppError(e)) return e;
	if (e instanceof Error) {
		return parseErrorMessage(e.message);
	}

	// rust에서 문자열로 넘어오는 경우
	if (typeof e === "string") {
		return parseErrorMessage(e);
	}
	return { kind: "Io", message: "알 수 없는 오류가 발생했습니다." };
}

export function showError(e: unknown): void {
	const err = normalizeError(e);
	toasts.showToast({type: "error", message: `${err.kind}: ${err.message}`});
}