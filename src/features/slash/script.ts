// 슬래시 커맨드 스크립트 엔진 (5-E-20-5-1·5-E-20-6-1, ADR-025). 앱을 모르는 순수 모듈 — 커맨드는 표로 받는다.
//
// 문법 (SillyTavern STscript와 같게 읽는다):
//   /이름 key=값 key="따옴표 값" key=[1,2] 이름 없는 인자   ← 커맨드 하나
//   커맨드 | 커맨드        앞 결과를 뒤로 (뒤에서 {{pipe}}, 이름 없는 인자가 없으면 그대로 넘김)
//   커맨드 || 커맨드       파이프 끊기 (뒤 커맨드는 앞 결과를 받지 않음)
//   {: 커맨드 | 커맨드 :}  묶음 (/if의 인자 등),  {: … :}() 는 그 자리에서 실행한 결과
//   /:세트.버튼            빠른 답장 부르기 (/run 세트.버튼)
//   // 주석, /# 주석, /* 블록 주석 */,  \| \{ \} \" 이스케이프
// 따옴표는 인자가 따옴표로 **시작할 때만** 묶는다 — 글 중간의 "는 그냥 글자 (SillyTavern과 같음).
// /로 시작하지 않는 조각은 오류 없이 건너뛴다 (SillyTavern과 같음).
// 매크로({{char}}·{{getvar::x}}·{{pipe}})는 그 커맨드를 실행하기 직전에 바꾼다.

export class ScriptError extends Error {}

/** 묶음 `{: … :}` — 실행할 때 다시 읽는다. immediate: `{: … :}()` (인자 자리에서 바로 실행) */
export interface Closure {
	kind: "closure";
	source: string;
	immediate?: boolean;
}
export type ArgValue = string | Closure;

export interface Command {
	name: string;
	named: Record<string, ArgValue>;
	/** 이름 없는 인자 (없으면 null → 앞 결과를 넘김) */
	unnamed: ArgValue | null;
	/** 앞이 `||` — 앞 결과를 받지 않음 */
	breakPipe?: boolean;
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

const isSpace = (c: string | undefined) => c === " " || c === "\t" || c === "\n" || c === "\r";

/** 위치 i가 이스케이프된 글자인가 (앞의 \ 개수가 홀수) */
function escaped(s: string, i: number): boolean {
	let n = 0;
	for (let j = i - 1; j >= 0 && s[j] === "\\"; j--) n++;
	return n % 2 === 1;
}

/** `\|` `\{` `\}` `\"` `\:` `\\` 풀기 */
function unescape(s: string): string {
	return s.replace(/\\([|{}"\\:])/g, "$1");
}

/** i에서 시작하는 묶음 `{: … :}`의 끝(:} 다음 위치). 안의 따옴표는 세지 않는다(글자일 수 있음) */
function closureEnd(s: string, i: number): number {
	let depth = 0;
	for (let j = i; j < s.length; j++) {
		if (escaped(s, j)) continue;
		if (s.startsWith("{:", j)) {
			depth++;
			j++;
		} else if (s.startsWith(":}", j)) {
			depth--;
			j++;
			if (depth === 0) return j + 1;
		}
	}
	throw new ScriptError("묶음 {: … :}이 닫히지 않음");
}

/** 따옴표로 시작하는 값 → [값, 다음 위치] */
function readQuoted(s: string, i: number): [string, number] {
	let v = "";
	let j = i + 1;
	for (; j < s.length; j++) {
		if (s[j] === "\\" && j + 1 < s.length) {
			v += s[j + 1];
			j++;
		} else if (s[j] === '"') return [v, j + 1];
		else v += s[j];
	}
	throw new ScriptError("따옴표가 닫히지 않음");
}

/** 값 하나 (이름 있는 인자의 값 또는 묶음) → [값, 다음 위치] */
function readValue(s: string, i: number): [ArgValue, number] {
	if (s.startsWith("{:", i)) {
		const end = closureEnd(s, i);
		const source = s.slice(i + 2, end - 2);
		if (s.startsWith("()", end)) return [{ kind: "closure", source, immediate: true }, end + 2];
		return [{ kind: "closure", source }, end];
	}
	if (s[i] === '"') return readQuoted(s, i);
	// 목록·객체 값 [..] {..}: 짝이 맞을 때까지 (안의 따옴표는 JSON이라 센다)
	if (s[i] === "[" || s[i] === "{") {
		let depth = 0;
		let quote = false;
		for (let j = i; j < s.length; j++) {
			const c = s[j];
			if (quote) {
				if (c === "\\") j++;
				else if (c === '"') quote = false;
				continue;
			}
			if (c === '"') quote = true;
			else if (c === "[" || c === "{") depth++;
			else if (c === "]" || c === "}") {
				depth--;
				if (depth === 0) return [s.slice(i, j + 1), j + 1];
			}
		}
	}
	let j = i;
	while (j < s.length && !isSpace(s[j]) && !(s[j] === "|" && !escaped(s, j))) j++;
	return [unescape(s.slice(i, j)), j];
}

/** 이름 없는 인자: 커맨드 끝(맨 바깥의 이스케이프 안 된 `|`)까지. 묶음 안의 `|`는 넘긴다. → [끝 위치] */
function rawEnd(s: string, i: number): number {
	let j = i;
	while (j < s.length) {
		if (s.startsWith("{:", j) && !escaped(s, j)) {
			j = closureEnd(s, j);
			continue;
		}
		if (s[j] === "|" && !escaped(s, j)) break;
		j++;
	}
	return j;
}

/** 커맨드 하나 읽기 (i는 `/` 자리) → [커맨드, 다음 위치] */
function readCommand(s: string, i: number): [Command, number] {
	const named: Record<string, ArgValue> = {};
	let unnamed: ArgValue | null = null;
	// /:세트.버튼 → /run 세트.버튼
	if (s.startsWith("/:", i)) {
		const [v, next] = s[i + 2] === '"' ? readQuoted(s, i + 2) : readValue(s, i + 2);
		return [{ name: "run", named, unnamed: v }, next];
	}
	const m = /^\/([^\s|]+)/.exec(s.slice(i));
	if (!m) throw new ScriptError("커맨드 이름이 없음");
	let j = i + m[0].length;
	for (;;) {
		while (j < s.length && isSpace(s[j])) j++;
		if (j >= s.length || (s[j] === "|" && !escaped(s, j))) break;
		const nm = /^([A-Za-z_][\w-]*)=/.exec(s.slice(j));
		if (nm) {
			const [v, next] = readValue(s, j + nm[0].length);
			named[nm[1]] = v;
			j = next;
			continue;
		}
		// 묶음은 이름 없는 인자로 받고, 뒤에 이름 있는 인자가 더 올 수 있다 (/if … {: … :} else={: … :})
		if (unnamed === null && s.startsWith("{:", j)) {
			[unnamed, j] = readValue(s, j);
			continue;
		}
		// 따옴표로 시작하고 그 따옴표로 끝나면 따옴표 안이 값, 아니면 커맨드 끝까지 글 그대로
		if (s[j] === '"') {
			try {
				const [v, next] = readQuoted(s, j);
				let k = next;
				while (k < s.length && isSpace(s[k])) k++;
				if (k >= s.length || (s[k] === "|" && !escaped(s, k))) {
					unnamed = v;
					j = next;
					continue;
				}
			} catch {
				/* 닫히지 않은 따옴표 → 글 그대로 */
			}
		}
		const end = rawEnd(s, j);
		unnamed = unescape(s.slice(j, end).trimEnd());
		j = end;
		break;
	}
	return [{ name: m[1].toLowerCase(), named, unnamed }, j];
}

/** 스크립트 읽기 */
export function parseScript(src: string): Command[] {
	const s = src;
	const out: Command[] = [];
	let breakNext = false;
	let i = 0;
	while (i < s.length) {
		if (isSpace(s[i])) {
			i++;
			continue;
		}
		if (s.startsWith("/*", i)) {
			const end = s.indexOf("*/", i + 2);
			i = end < 0 ? s.length : end + 2;
			continue;
		}
		if (s[i] === "|") {
			if (s[i + 1] === "|") {
				breakNext = true;
				i += 2;
			} else i++;
			continue;
		}
		// 주석: // 또는 /# — 다음 | 또는 줄 끝까지
		if (s.startsWith("//", i) || s.startsWith("/#", i)) {
			let j = i;
			while (j < s.length && s[j] !== "\n" && !(s[j] === "|" && !escaped(s, j))) j++;
			i = j;
			continue;
		}
		if (s[i] === "/" && i + 1 < s.length && !isSpace(s[i + 1])) {
			const [cmd, next] = readCommand(s, i);
			if (breakNext) cmd.breakPipe = true;
			breakNext = false;
			out.push(cmd);
			i = next;
			continue;
		}
		// /로 시작하지 않는 조각은 건너뜀 (SillyTavern과 같음)
		i = rawEnd(s, i);
	}
	return out;
}

/** 커맨드 하나만 읽기 (시험·도구용). 빈 것·주석이면 null */
export function parseCommand(raw: string): Command | null {
	return parseScript(raw)[0] ?? null;
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
			if (++steps > limit) throw new ScriptError(`실행 단계가 ${limit}번을 넘어 멈춤 (/${cmd.name})`);
			if (cmd.breakPipe) cur = "";
			const expand = async (v: ArgValue): Promise<ArgValue> => (typeof v === "string" ? expandMacros(v, cur, env.macro) : v.immediate ? run(v.source, cur) : v);
			const named: Record<string, ArgValue> = {};
			for (const [k, v] of Object.entries(cmd.named)) named[k] = await expand(v);
			const call: CommandCall = {
				named,
				unnamed: cmd.unnamed === null ? cur : await expand(cmd.unnamed),
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
