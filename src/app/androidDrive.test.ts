import { beforeEach, describe, expect, it, vi } from "vitest";
import { toasts } from "$shared/stores/toast.svelte";
import { AppError } from "$shared/ipc/client";
import { isDriveAuthRequired, NativeDriveError, runWithDriveAuth } from "./androidDrive";

describe("동기화 진행 토스트 및 드라이브 재인증 (1-J-2 · 1-J-3)", () => {
	beforeEach(() => {
		toasts.items = [];
	});

	it("진행률 토스트는 제자리에서 갱신되고 사용자가 누르면 닫힌 상태를 알린다", () => {
		let dismissed = false;
		const id = toasts.show(
			{
				tone: "info",
				message: "Syncing · 0/10",
				progress: { current: 0, total: 10 },
				dismissOnClick: true,
				onDismiss: () => {
					dismissed = true;
				},
				log: false,
			},
			0,
		);
		expect(toasts.items).toHaveLength(1);
		expect(toasts.update(id, { message: "Syncing · 5/10", progress: { current: 5, total: 10 } })).toBe(true);
		expect(toasts.items[0].progress).toEqual({ current: 5, total: 10 });

		// 사용자가 눌러서 닫음 → onDismiss 호출 및 이후 update는 false 반환
		toasts.dismiss(id, true);
		expect(dismissed).toBe(true);
		expect(toasts.items).toHaveLength(0);
		expect(toasts.update(id, { message: "Syncing · 6/10" })).toBe(false);
	});

	it("driveAuthRequired 및 needs_consent 오류를 재인증 대상으로 판별한다", () => {
		const backendErr = new AppError({
			code: "INVALID_INPUT",
			category: "VALIDATION",
			message: "Re-auth needed",
			retryable: false,
			details: { driveAuthRequired: true },
		});
		expect(isDriveAuthRequired(backendErr)).toBe(true);
		expect(isDriveAuthRequired(new NativeDriveError("needs_consent", "consent"))).toBe(true);
		expect(isDriveAuthRequired(new NativeDriveError("cancelled", "cancelled"))).toBe(false);
	});

	it("비드라이브 대상에서는 추가 인증 없이 바로 실행한다", async () => {
		const fn = vi.fn(async () => "ok");
		await expect(runWithDriveAuth(false, fn)).resolves.toBe("ok");
		expect(fn).toHaveBeenCalledWith(false);
	});
});
