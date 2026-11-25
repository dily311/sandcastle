// 게임 오류 코드 → 화면 알림.
// - GAME_ACTION_BLOCKED: 실패가 아니라 "지금은 안 됨"이라 경고로. 이유는 details.reason
// - GAME_NO_SESSION / GAME_NOT_OPENED: 플레이가 닫힘 → 게임 화면으로 돌아가는 버튼
// - GAME_SESSION_CHANGED: 기다리던 응답을 버렸다는 안내
// - GAME_DATA_INVALID: 시작 불가. details.diagnostics에 진단 목록
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import type { BlockReason } from "$ipc/bindings";
import { nav } from "$app/nav.svelte";
import { toAppError, type AppError } from "$shared/ipc/client";
import { toasts } from "$shared/stores/toast.svelte";

const t = (key: string) => get(_)(key);

export type { BlockReason };

/** GAME_ACTION_BLOCKED의 이유 (아니면 null) */
export function blockReason(e: unknown): BlockReason | null {
	const err = toAppError(e);
	if (err.code !== "GAME_ACTION_BLOCKED") return null;
	const reason = (err.details as { reason?: unknown } | undefined)?.reason;
	return typeof reason === "string" ? (reason as BlockReason) : null;
}

/** GAME_DATA_INVALID의 진단 목록 */
export function dataDiagnostics(e: unknown): { severity: string; text: string }[] {
	const err = toAppError(e);
	if (err.code !== "GAME_DATA_INVALID") return [];
	const list = (err.details as { diagnostics?: unknown } | undefined)?.diagnostics;
	return Array.isArray(list) ? (list as { severity: string; text: string }[]) : [];
}

/** 알림 모양만 정한다 (테스트용으로 분리) */
export function gameErrorToast(err: AppError, gameId?: string) {
	switch (err.code) {
		case "GAME_ACTION_BLOCKED":
			return { tone: "warning" as const, message: err.message };
		case "GAME_SESSION_CHANGED":
			return { tone: "warning" as const, message: t("errors.GAME_SESSION_CHANGED") };
		case "GAME_NO_SESSION":
		case "GAME_NOT_OPENED":
			return {
				tone: "warning" as const,
				message: t("errors.gameSessionClosed"),
				action: { label: t("errors.goToGame"), run: () => nav.go(gameId ? { page: "game", gameId } : { page: "game" }) },
			};
		default:
			return { tone: "error" as const, message: err.message, detail: err.code };
	}
}

/** call(..., { silent: true })로 부른 게임 커맨드의 오류를 알린다 */
export function reportGameError(e: unknown, gameId?: string) {
	toasts.show(gameErrorToast(toAppError(e), gameId));
}
