// 변수 정의를 화면에 쓰기 (5-F-4-3, ADR-029). 값은 늘 글자로 저장한다 (SillyTavern chat 변수와 같음).
import { tr } from "$lib/utils/i18n";
import type { VarDef } from "$features/character/library.svelte";

export interface VarRow {
	name: string;
	/** 대화에 들어 있는 값 (없으면 undefined — 정의의 처음 값을 흐리게 보인다) */
	value: string | undefined;
	def?: VarDef;
}

/** 정의된 변수를 정의 순서대로 먼저(값이 없어도), 그 뒤에 정의 없는 변수를 이름순으로 */
export function varRows(vars: Record<string, string>, defs: readonly VarDef[] = []): VarRow[] {
	const defined = new Set(defs.map((d) => d.key));
	return [
		...defs.map((d) => ({ name: d.key, value: vars[d.key], def: d })),
		...Object.keys(vars)
			.filter((k) => !defined.has(k))
			.sort((a, b) => a.localeCompare(b))
			.map((k) => ({ name: k, value: vars[k] })),
	];
}

/** 정의에 맞는 값으로 (수는 최소·최대 안으로, 참거짓은 true/false). 맞출 수 없으면 error */
export function coerceVar(def: VarDef | undefined, raw: string): { value: string } | { error: string } {
	if (!def || def.kind === "string") return { value: raw };
	if (def.kind === "boolean") {
		const v = raw.trim().toLowerCase();
		if (v === "true" || v === "false") return { value: v };
		return { error: tr("statusUi.errBool", { key: def.key }) };
	}
	const n = Number(raw.trim());
	if (raw.trim() === "" || !Number.isFinite(n)) return { error: tr("statusUi.errNum", { key: def.key }) };
	const clamped = Math.min(def.max ?? Infinity, Math.max(def.min ?? -Infinity, n));
	return { value: String(clamped) };
}

/** 툴팁: 설명과 AI 권한 */
export function varTitle(def: VarDef | undefined): string | undefined {
	if (!def) return undefined;
	const ai = def.aiWrite ? tr("statusUi.aiReadWrite") : def.aiRead ? tr("statusUi.aiReadOnly") : tr("statusUi.aiHidden");
	const range = def.kind === "number" && (def.min != null || def.max != null) ? ` · ${def.min ?? "…"}~${def.max ?? "…"}` : "";
	return [def.description, `${ai}${range}`].filter(Boolean).join("\n");
}
