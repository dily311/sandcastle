import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { initI18n, setAppLocale } from "$lib/utils/i18n";
import { toasts } from "$shared/stores/toast.svelte";
import {
	beginStProgress,
	endStProgress,
	handleStProgressEvent,
	handleSyncProgressEvent,
	resetBgProgressForTest,
} from "./syncProgress.svelte";

describe("syncProgress · SillyTavern & Sync 진행률 토스트·Android 포그라운드 알림", () => {
	const nativeMessages: string[] = [];

	beforeAll(async () => {
		await initI18n();
		await setAppLocale("ko");
	});

	beforeEach(() => {
		nativeMessages.length = 0;
		(window as unknown as { SandcastleNative?: { postMessage(msg: string): void } }).SandcastleNative = {
			postMessage: (msg: string) => nativeMessages.push(msg),
		};
		resetBgProgressForTest();
		toasts.items = [];
	});

	it("SillyTavern 내보내기·가져오기 진행률을 토스트 하나와 Android 포그라운드 알림에 실시간 반영하고 완료 시 닫는다", () => {
		beginStProgress({ kind: "export", initialPhase: "preparing" });
		expect(toasts.items).toHaveLength(1);
		expect(toasts.items[0].dismissOnClick).toBe(true);
		expect(toasts.items[0].progress).toEqual({ indeterminate: true });
		expect(nativeMessages.some((m) => m.startsWith("sync:progress:"))).toBe(true);

		handleStProgressEvent({ kind: "export", phase: "chats", current: 3, total: 10 });
		expect(toasts.items).toHaveLength(1);
		expect(toasts.items[0].message).toContain("3/10 · 30%");
		expect(toasts.items[0].progress).toEqual({ current: 3, total: 10 });

		handleStProgressEvent({ kind: "export", phase: "finishing", current: 10, total: 10 });
		expect(toasts.items[0].progress).toEqual({ current: 10, total: 10 });
		const lastProgressMsg = [...nativeMessages].reverse().find((m) => m.startsWith("sync:progress:"));
		expect(lastProgressMsg).toBeDefined();
		const parsed = JSON.parse(lastProgressMsg!.slice("sync:progress:".length));
		expect(parsed).toMatchObject({ current: 10, total: 10, indeterminate: false });

		endStProgress();
		expect(toasts.items).toHaveLength(0);
		expect(nativeMessages.at(-1)).toBe("sync:stop");
	});

	it("사용자가 진행률 토스트를 클릭해 닫으면 이후 진행 중에는 토스트가 다시 뜨지 않고 Android 알림만 계속 갱신된다", () => {
		beginStProgress({ kind: "import", initialPhase: "extracting" });
		expect(toasts.items).toHaveLength(1);
		const id = toasts.items[0].id;

		// 사용자가 토스트 본문을 눌러 닫음 (byUser = true)
		toasts.dismiss(id, true);
		expect(toasts.items).toHaveLength(0);

		nativeMessages.length = 0;
		handleStProgressEvent({ kind: "import", phase: "characters", current: 5, total: 5 });
		// 토스트는 다시 나타나지 않지만 Android 알림은 100%로 갱신됨
		expect(toasts.items).toHaveLength(0);
		expect(nativeMessages.some((m) => m.startsWith("sync:progress:"))).toBe(true);

		endStProgress();
		expect(nativeMessages.at(-1)).toBe("sync:stop");

		// 다음 새 작업에서는 다시 정상적으로 토스트가 표시된다
		beginStProgress({ kind: "import", initialPhase: "preparing" });
		expect(toasts.items).toHaveLength(1);
		endStProgress();
	});

	it("동기화와 SillyTavern 작업이 겹칠 때 한쪽이 끝나도 남은 작업의 Android 포그라운드 알림을 유지한다", () => {
		vi.useFakeTimers();
		try {
			handleSyncProgressEvent({ phase: "downloading", step: "downloading", current: 2, total: 8 });
			beginStProgress({ kind: "import", initialPhase: "chats" });
			handleStProgressEvent({ kind: "import", phase: "chats", current: 4, total: 4 });
			expect(toasts.items).toHaveLength(2);

			// SillyTavern 작업이 먼저 끝나도 동기화가 진행 중이므로 sync:stop 대신 동기화 알림이 유지된다
			endStProgress();
			expect(toasts.items).toHaveLength(1);
			expect(nativeMessages.at(-1)).toContain("sync:progress:");
			expect(nativeMessages.at(-1)).toContain("\"current\":2");

			handleSyncProgressEvent({ phase: "done", step: "", current: null, total: null });
			expect(toasts.items).toHaveLength(0);
			expect(nativeMessages.at(-1)).toBe("sync:stop");
		} finally {
			vi.useRealTimers();
		}
	});
});
