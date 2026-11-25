import { describe, expect, it, vi } from "vitest";

vi.mock("$app/nav.svelte", () => ({ nav: { go: vi.fn() } }));

import { setAppLocale } from "$lib/utils/i18n";
import { AppError } from "$shared/ipc/client";
import { blockReason, dataDiagnostics, gameErrorToast } from "./errors";

setAppLocale("ko");

const err = (code: string, details?: unknown) =>
	new AppError({ code: code as never, category: "GAME", message: "메시지", retryable: false, details });

describe("게임 오류 코드", () => {
	it("막힌 행동은 이유를 꺼내고 경고로 알린다", () => {
		const e = err("GAME_ACTION_BLOCKED", { reason: "notPresent" });
		expect(blockReason(e)).toBe("notPresent");
		expect(gameErrorToast(e).tone).toBe("warning");
		expect(blockReason(err("INVALID_INPUT"))).toBeNull();
	});

	it("플레이가 닫혔으면 돌아가는 버튼을 준다", () => {
		const t = gameErrorToast(err("GAME_NO_SESSION"), "g1");
		expect(t.tone).toBe("warning");
		expect("action" in t && t.action?.label).toBe("게임으로");
	});

	it("데이터 오류의 진단 목록을 꺼낸다", () => {
		const list = [{ severity: "error", text: "없는 씬" }];
		expect(dataDiagnostics(err("GAME_DATA_INVALID", { diagnostics: list }))).toEqual(list);
		expect(gameErrorToast(err("GAME_DATA_INVALID")).tone).toBe("error");
	});
});
