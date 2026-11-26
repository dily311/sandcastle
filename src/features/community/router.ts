// 플러그인 iframe에서 온 메시지를 거른다 (CLAUDE.md 6-D-2, ADR-012). 순수 함수라 테스트한다.
// 받는 것은 {type:"sc-plugin-call", id, method, args}뿐. 보낸 창이 어느 플러그인의 iframe인지는 호스트가 알려 준다
// (메시지 안의 값은 믿지 않음). 메서드 표에 없거나 허락받지 않은 권한이 필요하면 IPC를 부르지 않고 거절한다.
// 백엔드(call_user_plugin)도 같은 표로 다시 확인한다.
import { get } from "svelte/store";
import { _ } from "svelte-i18n";

const tr = (key: string, values?: Record<string, string | number>) => get(_)(key, values ? { values } : undefined);

export type PluginCall = { id: number; method: string; args: unknown };
export type Route = { ok: true; call: PluginCall } | { ok: false; id: number | null; code: string; message: string } | null;

const MAX_METHOD = 64;
/** 인자 크기 상한 (저장소 값 64KB + 여유) */
export const MAX_ARGS_CHARS = 80_000;

/** null = 플러그인 호출 메시지가 아님(무시) */
export function route(data: unknown, api: Record<string, string | null>, granted: readonly string[]): Route {
	if (!data || typeof data !== "object") return null;
	const d = data as Record<string, unknown>;
	if (d.type !== "sc-plugin-call") return null;
	const id = typeof d.id === "number" && Number.isSafeInteger(d.id) ? d.id : null;
	const method = typeof d.method === "string" ? d.method : "";
	const fail = (code: string, message: string): Route => ({ ok: false, id, code, message });
	if (id === null) return fail("INVALID_INPUT", tr("communityUi.routerNoCallId"));
	if (!method || method.length > MAX_METHOD || !Object.hasOwn(api, method)) return fail("INVALID_INPUT", tr("communityUi.routerUnknownMethod", { method }));
	const need = api[method];
	if (need && !granted.includes(need)) return fail("INVALID_INPUT", tr("communityUi.routerMissingPerm", { method, need }));
	let size = 0;
	try {
		size = JSON.stringify(d.args ?? null).length;
	} catch {
		return fail("INVALID_INPUT", tr("communityUi.routerBadJson"));
	}
	if (size > MAX_ARGS_CHARS) return fail("INVALID_INPUT", tr("communityUi.routerArgsTooLarge"));
	return { ok: true, call: { id, method, args: d.args ?? null } };
}
