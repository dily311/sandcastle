// 슬래시 커맨드 스크립트 엔진 (5-E-20-5-1, ADR-025). 앱을 모르는 순수 모듈 — 커맨드는 표로 받는다.
//
// 문법 (SillyTavern STscript의 일부):
//   /이름 key=값 key="따옴표 값" 이름 없는 인자     ← 커맨드 하나
//   커맨드 | 커맨드                                  ← 앞 결과를 뒤로 (뒤에서 {{pipe}}, 이름 없는 인자가 없으면 그대로 넘김)
//   {: 커맨드 | 커맨드 :}                            ← 묶음 (/if의 인자 등)
//   \| \{ \}  이스케이프,  빈 커맨드(||)·// 주석은 건너뜀
// 매크로({{char}}·{{getvar::x}}·{{pipe}})는 그 커맨드를 실행하기 직전에 바꾼다.

export class ScriptError extends Error {}

/** 묶음 `{: … :}` — 실행할 때 다시 읽는다 */
export interface Closure {
	kind: "closure";
	source: string;
}
export type ArgValue = string | Closure;

export interface Command {
	name: string;
	named: Record<string, ArgValue>;
	/** 이름 없는 인자 (없으면 null → 앞 결과를 넘김) */
	unnamed: ArgValue | null;
}

export interface CommandCall {
	/** 매크로를 바꾼 이름 있는 인자 (묶음은 그대로) */
	named: Record<string, ArgValue>;
	/** 매크로를 바꾼 이름 없는 인자 (없으면 앞 결과) */
	unnamed: ArgValue;
	/** 앞 커맨드의 결과 */
	pipe: string;
	/** 묶음 실행 (앞 결과를 이어받음). 결과는 묶음 안 마지막 결과 */
	run: (c: ArgValue) => Promise<string>;
}
/** 커맨드 처리. 돌려준 글이 다음 커맨드의 pipe (undefined면 pipe 그대로) */
export type Handler = (call: CommandCall) => Promise<string | void> | string | void;

export interface ScriptEnv {
	commands: Record<string, Handler>;
	/** {{이름}}·{{이름::인자}} 값. 모르면 undefined (그대로 둠) */
	macro?: (name: string, arg: string | undefined) => string | undefined;
	/** 아는 커맨드지만 지원하지 않는 것 (SillyTavern에서 온 스크립트에 친절한 오류) */
	unsupported?: string[];
	/** 실행 단계 상한 (기본 500) */
	maxSteps?: number;
}

export interface ScriptResult {
	pipe: string;
	aborted: boolean;
}

// ---------------------------------------------------------------------------
// 읽기
// ---------------------------------------------------------------------------

/** 맨 바깥의 `|`로 나눈다 (따옴표·묶음 안, `\|`는 나누지 않음) */
export function splitPipes(src: string): string[] {
	const out: string[] = [];
	let cur = "";
	let depth = 0;
	let quote = false;
	for (let i = 0; i < src.length; i++) {
		const c = src[i];
		const two = src.slice(i, i + 2);
		if (c === "\\" && i + 1 < src.length) {
			cur += two;
			i++;
			continue;
		}
		if (!quote && two === "{:") {
			depth++;
			cur += two;
			i++;
			continue;
		}
		if (!quote && depth > 0 && two === ":}") {
			depth--;
			cur += two;
			i++;
			continue;
		}
		if (c === '"' && depth === 0) quote = !quote;
		if (c === "|" && depth === 0 && !quote) {
			out.push(cur);
			cur = "";
			continue;
		}
		cur += c;
	}
	if (depth > 0) throw new ScriptError("묶음 {: … :}이 닫히지 않음");
	out.push(cur);
	return out;
}

/** `\|` 같은 이스케이프 풀기 */
function unescape(s: string): string {
	return s.replace(/\\([|{}\\"])/g, "$1");
}

/** 위치 i의 값 하나를 읽는다 → [값, 다음 위치] */
function readValue(s: string, i: number): [ArgValue, number] {
	if (s.startsWith("{:", i)) {
		let depth = 0;
		let quote = false;
		for (let j = i; j < s.length; j++) {
			if (s[j] === "\\") {
				j++;
				continue;
			}
			if (s[j] === '"') quote = !quote;
			if (quote) continue;
			if (s.startsWith("{:", j)) {
				depth++;
				j++;
			} else if (s.startsWith(":}", j)) {
				depth--;
				j++;
				if (depth === 0) return [{ kind: "closure", source: s.slice(i + 2, j - 1) }, j + 1];
			}
		}
		throw new ScriptError("묶음 {: … :}이 닫히지 않음");
	}
	if (s[i] === '"') {
		let j = i + 1;
		let v = "";
		for (; j < s.length && s[j] !== '"'; j++) {
			if (s[j] === "\\" && j + 1 < s.length) {
				v += s[j + 1];
				j++;
			} else v += s[j];
		}
		return [v, j + 1];
	}
	let j = i;
	while (j < s.length && !/\s/.test(s[j])) j++;
	return [unescape(s.slice(i, j)), j];
}

/** 커맨드 하나 읽기. 빈 것·주석이면 null */
export function parseCommand(raw: string): Command | null {
	const s = raw.trim();
	if (!s || s.startsWith("//")) return null;
	const m = /^\/([^\s|]+)/.exec(s);
	if (!m) throw new ScriptError(`커맨드는 /로 시작해야 함: ${s.slice(0, 30)}`);
	const named: Record<string, ArgValue> = {};
	let unnamed: ArgValue | null = null;
	let i = m[0].length;
	for (;;) {
		while (i < s.length && /\s/.test(s[i])) i++;
		if (i >= s.length) break;
		const nm = /^([A-Za-z_][\w-]*)=/.exec(s.slice(i));
		if (nm) {
			const [v, next] = readValue(s, i + nm[0].length);
			named[nm[1]] = v;
			i = next;
			continue;
		}
		// 묶음은 이름 없는 인자로 받고, 뒤에 이름 있는 인자가 더 올 수 있다 (/if … {: … :} else={: … :})
		if (unnamed === null && s.startsWith("{:", i)) {
			[unnamed, i] = readValue(s, i);
			continue;
		}
		// 나머지 글 전체가 이름 없는 인자 (따옴표 하나로 감싼 것이면 따옴표를 벗김)
		const rest = s.slice(i).trim();
		const [v, next] = rest.startsWith('"') ? readValue(rest, 0) : [null, 0];
		unnamed = v !== null && next >= rest.length ? v : unescape(rest);
		break;
	}
	return { name: m[1].toLowerCase(), named, unnamed };
}

export function parseScript(src: string): Command[] {
	return splitPipes(src)
		.map(parseCommand)
		.filter((c): c is Command => c !== null);
}

// ---------------------------------------------------------------------------
// 실행
// ---------------------------------------------------------------------------

class Abort {}

/** {{이름}}·{{이름::인자}} 바꾸기 */
export function expandMacros(text: string, pipe: string, macro?: ScriptEnv["macro"]): string {
	return text.replace(/\{\{([^{}]+?)\}\}/g, (all, inner: string) => {
		const [name, ...rest] = inner.split("::");
		const key = name.trim().toLowerCase();
		if (key === "pipe") return pipe;
		return macro?.(key, rest.length ? rest.join("::") : undefined) ?? all;
	});
}

const num = (s: string) => (s.trim() !== "" && !Number.isNaN(Number(s)) ? Number(s) : null);

/** /if 비교 */
export function compare(left: string, right: string, rule: string): boolean {
	const [a, b] = [num(left), num(right)];
	const both = a !== null && b !== null;
	switch (rule) {
		case "eq":
			return both ? a === b : left === right;
		case "neq":
			return both ? a !== b : left !== right;
		case "lt":
			return both ? a < b : left < right;
		case "gt":
			return both ? a > b : left > right;
		case "lte":
			return both ? a <= b : left <= right;
		case "gte":
			return both ? a >= b : left >= right;
		case "in":
			return left.includes(right);
		case "nin":
			return !left.includes(right);
		case "not":
			return !left || left === "false" || left === "0";
		default:
			throw new ScriptError(`/if: 모르는 비교 rule=${rule} (eq·neq·lt·gt·lte·gte·in·nin·not)`);
	}
}

export async function runScript(src: string, env: ScriptEnv, pipe = ""): Promise<ScriptResult> {
	const limit = env.maxSteps ?? 500;
	let steps = 0;

	const run = async (source: string, input: string): Promise<string> => {
		let cur = input;
		for (const cmd of parseScript(source)) {
			if (++steps > limit) throw new ScriptError(`실행 단계가 ${limit}번을 넘어 멈춤`);
			const expand = (v: ArgValue): ArgValue => (typeof v === "string" ? expandMacros(v, cur, env.macro) : v);
			const named = Object.fromEntries(Object.entries(cmd.named).map(([k, v]) => [k, expand(v)]));
			const call: CommandCall = {
				named,
				unnamed: cmd.unnamed === null ? cur : expand(cmd.unnamed),
				pipe: cur,
				run: (c) => (typeof c === "string" ? Promise.resolve(c) : run(c.source, cur)),
			};
			const out = await (builtins[cmd.name] ?? env.commands[cmd.name] ?? missing(cmd.name))(call);
			if (out !== undefined) cur = out;
		}
		return cur;
	};

	const missing = (name: string): Handler => {
		if (env.unsupported?.includes(name)) throw new ScriptError(`/${name}은 지원하지 않음`);
		throw new ScriptError(`모르는 커맨드: /${name}`);
	};

	const builtins: Record<string, Handler> = {
		abort: () => {
			throw new Abort();
		},
		if: async ({ named, unnamed, run: exec }) => {
			const text = (v: ArgValue | undefined) => (typeof v === "string" ? v : "");
			const ok = compare(text(named.left), text(named.right), text(named.rule) || "eq");
			const branch = ok ? unnamed : named.else;
			return branch === undefined ? undefined : exec(branch);
		},
	};

	try {
		return { pipe: await run(src, pipe), aborted: false };
	} catch (e) {
		if (e instanceof Abort) return { pipe: "", aborted: true };
		throw e;
	}
}
