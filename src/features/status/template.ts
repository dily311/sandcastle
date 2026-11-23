// 상태창 틀 채우기 (5-F-4-5, ADR-029 ④나): 캐릭터의 HTML 틀에 변수 값을 넣는다 — AI가 상태창 글을 쓰지 않아도 된다.
// 매크로: {{getvar::x}}(이 대화, 없으면 정의의 처음 값) · {{getglobalvar::x}} · {{char}} · {{user}}. 모르는 매크로는 그대로 둔다.
// 값은 AI가 바꿀 수도 있으므로 HTML로 해석되지 않게 이스케이프한다 (틀의 HTML만 HTML)
import type { VarDef } from "$features/character/library.svelte";

export interface TemplateData {
	vars: Record<string, unknown>;
	globals: Record<string, unknown>;
	defs?: readonly VarDef[];
	char?: string;
	user?: string;
}

const ESC: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };
const escape = (s: string) => s.replace(/[&<>"']/g, (c) => ESC[c]);
const text = (v: unknown) => (v == null ? "" : typeof v === "string" ? v : JSON.stringify(v));

export function fillTemplate(template: string, d: TemplateData): string {
	return template.replace(/\{\{\s*([^{}]+?)\s*\}\}/g, (whole, raw: string) => {
		const m = /^(getvar|getglobalvar)::(.+)$/i.exec(raw);
		if (m) {
			const name = m[2].trim();
			if (m[1].toLowerCase() === "getglobalvar") return escape(text(d.globals[name]));
			const v = name in d.vars ? d.vars[name] : d.defs?.find((x) => x.key === name)?.defaultValue;
			return escape(text(v));
		}
		const key = raw.toLowerCase();
		if (key === "char") return escape(d.char ?? "");
		if (key === "user") return escape(d.user ?? "");
		return whole;
	});
}
