import { describe, expect, it } from "vitest";
import { MAX_ARGS_CHARS, route } from "./router";

const api = { log: null, "storage.get": "storage", "app.info": null };

describe("플러그인 메시지 거르기", () => {
	it("플러그인 호출이 아니면 무시", () => {
		expect(route(null, api, [])).toBeNull();
		expect(route("x", api, [])).toBeNull();
		expect(route({ type: "sc-html-height", h: 3 }, api, [])).toBeNull();
	});

	it("허락한 권한의 메서드만 통과", () => {
		expect(route({ type: "sc-plugin-call", id: 1, method: "log", args: { message: "a" } }, api, [])).toEqual({
			ok: true,
			call: { id: 1, method: "log", args: { message: "a" } },
		});
		const denied = route({ type: "sc-plugin-call", id: 2, method: "storage.get", args: {} }, api, []);
		expect(denied).toMatchObject({ ok: false, id: 2 });
		expect(route({ type: "sc-plugin-call", id: 3, method: "storage.get", args: {} }, api, ["storage"])).toMatchObject({ ok: true });
	});

	it("모르는 메서드·프로토타입 이름·id 없음·큰 인자는 거절", () => {
		expect(route({ type: "sc-plugin-call", id: 4, method: "fs.read" }, api, [])).toMatchObject({ ok: false });
		expect(route({ type: "sc-plugin-call", id: 5, method: "toString" }, api, [])).toMatchObject({ ok: false });
		expect(route({ type: "sc-plugin-call", id: 6, method: "__proto__" }, api, [])).toMatchObject({ ok: false });
		expect(route({ type: "sc-plugin-call", method: "log" }, api, [])).toMatchObject({ ok: false, id: null });
		expect(route({ type: "sc-plugin-call", id: 7, method: "log", args: "x".repeat(MAX_ARGS_CHARS) }, api, [])).toMatchObject({ ok: false });
	});
});
