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
	/** 만들어진 자리의 변수 범위 (실행할 때 붙임 — 묶음은 만든 자리의 변수를 본다) */
	scope?: Scope;
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
	/** 실행 단계 상한 (기본 1만, ADR-025 5-E-20-6-0) */
	maxSteps?: number;
	/** 반복 하나의 바퀴 상한 (기본 1000) */
	maxLoop?: number;
	/** /run 세트.버튼 · /:세트.버튼 → 그 빠른 답장의 스크립트 (없으면 undefined) */
	quickReply?: (name: string) => string | undefined;
	/** /if·/while의 left·right가 변수 이름일 때 대화·전역 변수 값 (범위 변수는 엔진이 먼저 본다) */
	variable?: (name: string) => string | undefined;
}

export const DEFAULT_MAX_STEPS = 10_000;
export const DEFAULT_MAX_LOOP = 1000;

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
		const [v, after] = s[i + 2] === '"' ? readQuoted(s, i + 2) : readValue(s, i + 2);
		// 뒤의 key=값은 묶음 인자 (/:f a=7)
		let j = after;
		for (;;) {
			let k = j;
			while (k < s.length && isSpace(s[k])) k++;
			const nm = /^([A-Za-z_][\w-]*)=/.exec(s.slice(k));
			if (!nm) break;
			const [val, next] = readValue(s, k + nm[0].length);
			named[nm[1]] = val;
			j = next;
		}
		return [{ name: "run", named, unnamed: v }, j];
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
/** /break: 가장 가까운 반복(또는 /run 묶음)을 끝냄 */
class Break {
	constructor(readonly value: string) {}
}
/** /return: /run으로 부른 묶음(또는 스크립트)을 끝내고 값을 돌려줌 */
class Return {
	constructor(readonly value: string) {}
}

/** 범위 변수 (/let·/var, 묶음마다 하나. 바깥 범위를 본다) */
export class Scope {
	private vars = new Map<string, ArgValue>();
	constructor(readonly parent?: Scope) {}
	private owner(name: string): Scope | undefined {
		return this.vars.has(name) ? this : this.parent?.owner(name);
	}
	has(name: string): boolean {
		return !!this.owner(name);
	}
	get(name: string): ArgValue | undefined {
		return this.owner(name)?.vars.get(name);
	}
	/** /let: 이 범위에 새로 */
	let(name: string, v: ArgValue) {
		this.vars.set(name, v);
	}
	/** /var: 있는 범위에 바꾸고, 없으면 이 범위에 */
	set(name: string, v: ArgValue) {
		(this.owner(name) ?? this).vars.set(name, v);
	}
}

/** 목록·객체 값에서 index로 꺼내기 (JSON이 아니면 undefined) */
export function getIndexed(value: string, index: string): string | undefined {
	try {
		const v = JSON.parse(value) as unknown;
		const got = Array.isArray(v) ? v[Number(index)] : v && typeof v === "object" ? (v as Record<string, unknown>)[index] : undefined;
		return got === undefined ? undefined : typeof got === "string" ? got : JSON.stringify(got);
	} catch {
		return undefined;
	}
}

/** 목록·객체 값의 index 자리를 바꾼 새 JSON (원래 값이 비었으면 index가 숫자면 목록, 아니면 객체) */
export function setIndexed(value: string, index: string, item: string): string {
	let v: unknown;
	try {
		v = value.trim() ? JSON.parse(value) : null;
	} catch {
		v = null;
	}
	const parsed = (() => {
		try {
			return JSON.parse(item) as unknown;
		} catch {
			return item;
		}
	})();
	if (Array.isArray(v) && /^\d+$/.test(index)) {
		v[Number(index)] = parsed;
		return JSON.stringify(v);
	}
	const obj = v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, unknown>) : /^\d+$/.test(index) && v === null ? null : {};
	if (obj === null) {
		const list: unknown[] = [];
		list[Number(index)] = parsed;
		return JSON.stringify(list);
	}
	obj[index] = parsed;
	return JSON.stringify(obj);
}

/** /addvar: 목록이면 덧붙이고, 둘 다 숫자면 더하고, 아니면 글자를 잇는다 */
export function addTo(value: string, add: string): string {
	try {
		const v = JSON.parse(value) as unknown;
		if (Array.isArray(v)) {
			let item: unknown = add;
			try {
				item = JSON.parse(add);
			} catch {
				/* 글자 그대로 */
			}
			return JSON.stringify([...v, item]);
		}
	} catch {
		/* 목록 아님 */
	}
	const [a, b] = [num(value), num(add)];
	return a !== null && b !== null ? String(a + b) : value + add;
}

/** 묶음 앞머리의 인자 선언 (`{: a=1 b="x" /echo … :}`) → [선언, 본문] */
export function closureParams(source: string): [Record<string, ArgValue>, string] {
	const params: Record<string, ArgValue> = {};
	let i = 0;
	for (;;) {
		while (i < source.length && isSpace(source[i])) i++;
		const m = /^([A-Za-z_][\w-]*)=/.exec(source.slice(i));
		if (!m) break;
		const [v, next] = readValue(source, i + m[0].length);
		params[m[1]] = v;
		i = next;
	}
	return [params, source.slice(i)];
}

/** "이름 값" → [이름, 값] (값이 묶음이면 묶음으로) */
function nameAndValue(text: string): [string, ArgValue | undefined] {
	const m = /^\s*(\S+)\s*/.exec(text);
	if (!m) return ["", undefined];
	const rest = text.slice(m[0].length);
	if (!rest) return [m[1], undefined];
	if (rest.startsWith("{:")) return [m[1], readValue(rest, 0)[0]];
	if (rest.startsWith('"')) {
		try {
			const [v, next] = readQuoted(rest, 0);
			if (!rest.slice(next).trim()) return [m[1], v];
		} catch {
			/* 글 그대로 */
		}
	}
	return [m[1], rest];
}

/** {{이름}}·{{이름::인자}} 바꾸기. 글 안의 묶음 `{: … :}`은 그대로 둔다 (그 묶음이 실행될 때 바꾼다 — /times 3 {: … {{timesIndex}} … :}) */
export function expandMacros(text: string, pipe: string, macro?: ScriptEnv["macro"]): string {
	if (text.includes("{:")) {
		let out = "";
		let i = 0;
		for (let j = 0; j < text.length; j++) {
			if (text.startsWith("{:", j) && !escaped(text, j)) {
				let end: number;
				try {
					end = closureEnd(text, j);
				} catch {
					break; // 닫히지 않은 묶음: 남은 글은 보통 글처럼
				}
				out += expandPlain(text.slice(i, j), pipe, macro) + text.slice(j, end);
				i = end;
				j = end - 1;
			}
		}
		return out + expandPlain(text.slice(i), pipe, macro);
	}
	return expandPlain(text, pipe, macro);
}

function expandPlain(text: string, pipe: string, macro?: ScriptEnv["macro"]): string {
	text = text.replace(/\s*\{\{trim\}\}\s*/gi, "");
	return text.replace(/\{\{([^{}]+?)\}\}/g, (all, inner: string) => {
		const [name, ...rest] = inner.split("::");
		// 이름만 소문자로 ({{random:가,나}}처럼 이름 뒤에 붙은 값은 그대로)
		const raw = name.trim();
		const colon = raw.indexOf(":");
		const key = colon < 0 ? raw.toLowerCase() : raw.slice(0, colon).toLowerCase() + raw.slice(colon);
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

const asText = (v: ArgValue | undefined) => (typeof v === "string" ? v : "");

/** 주사위: "d6", "2d6", "1d20+3" → 합 (모양이 아니면 null) */
export function roll(spec: string, rnd: () => number = Math.random): number | null {
	const m = /^\s*(\d*)\s*d\s*(\d+)\s*([+-]\s*\d+)?\s*$/i.exec(spec);
	if (!m) return /^\s*\d+\s*$/.test(spec) ? Math.floor(rnd() * Number(spec)) + 1 : null;
	const [n, sides] = [Math.min(100, Number(m[1] || 1)), Number(m[2])];
	if (!sides) return null;
	let sum = Number((m[3] ?? "0").replace(/\s/g, ""));
	for (let i = 0; i < n; i++) sum += Math.floor(rnd() * sides) + 1;
	return sum;
}

/** {{random::a::b}} · {{random:a,b}} 고를 것들 */
function choices(arg: string | undefined, raw: string): string[] {
	if (arg !== undefined) return arg.split("::");
	const legacy = /^\w+:(.*)$/s.exec(raw);
	return legacy ? legacy[1].split(",") : [];
}

const pad = (n: number) => String(n).padStart(2, "0");
/** 시간 매크로 (기기 시각) */
function timeMacro(name: string, now = new Date()): string | undefined {
	switch (name) {
		case "time":
			return now.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
		case "date":
			return now.toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" });
		case "weekday":
			return now.toLocaleDateString(undefined, { weekday: "long" });
		case "isotime":
			return `${pad(now.getHours())}:${pad(now.getMinutes())}`;
		case "isodate":
			return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
		default:
			return undefined;
	}
}

/** 문장 끝 (. ! ? … 。 와 그 뒤 따옴표·괄호) */
const SENTENCE_END = /[.!?…。！？]["'”’」』)\]]*/g;
/** /trimend: 마지막 온전한 문장까지 */
export function trimEnd(text: string): string {
	let last = -1;
	for (const m of text.matchAll(SENTENCE_END)) last = (m.index ?? 0) + m[0].length;
	return last > 0 ? text.slice(0, last).trimEnd() : text.trimEnd();
}
/** /trimstart: 첫 온전한 문장부터 (앞의 잘린 문장 조각을 뺌) */
export function trimStart(text: string): string {
	const first = SENTENCE_END.exec(text);
	SENTENCE_END.lastIndex = 0;
	if (!first || first.index === undefined) return text.trimStart();
	const rest = text.slice(first.index + first[0].length).trimStart();
	return rest || text.trimStart();
}

/** 글·목록·객체의 길이 (/len) */
export function lengthOf(value: string): number {
	try {
		const v = JSON.parse(value) as unknown;
		if (Array.isArray(v)) return v.length;
		if (v && typeof v === "object") return Object.keys(v).length;
	} catch {
		/* 글 */
	}
	return [...value].length;
}

export async function runScript(src: string, env: ScriptEnv, pipe = ""): Promise<ScriptResult> {
	const limit = env.maxSteps ?? DEFAULT_MAX_STEPS;
	const loopLimit = env.maxLoop ?? DEFAULT_MAX_LOOP;
	let steps = 0;
	const root = new Scope();

	/** 엔진이 먼저 보는 매크로: 범위 변수·묶음 인자·반복 번호 */
	const macroIn = (scope: Scope) => (name: string, arg: string | undefined) => {
		if ((name === "var" || name === "arg") && arg !== undefined) {
			const [n, index] = arg.split("::");
			const v = asText(scope.get(n.trim()));
			return index !== undefined ? (getIndexed(v, index.trim()) ?? "") : v;
		}
		if (name === "timesindex") return asText(scope.get("timesIndex"));
		if (name === "random" || name === "pick" || name.startsWith("random:") || name.startsWith("pick:")) {
			const list = choices(arg, name);
			return list.length ? list[Math.floor(Math.random() * list.length)] : "";
		}
		if (name === "roll" || name.startsWith("roll:")) {
			const r = roll(arg ?? name.slice(5));
			return r === null ? "" : String(r);
		}
		if (name === "noop") return "";
		return timeMacro(name) ?? env.macro?.(name, arg);
	};

	/** 스크립트 본문 실행 */
	const run = async (source: string, input: string, scope: Scope): Promise<string> => {
		let cur = input;
		for (const cmd of parseScript(source)) {
			if (++steps > limit) throw new ScriptError(`실행 단계가 ${limit}번을 넘어 멈춤 (/${cmd.name})`);
			if (cmd.breakPipe) cur = "";
			const pipeNow = cur;
			const expand = async (v: ArgValue): Promise<ArgValue> =>
				typeof v === "string" ? expandMacros(v, pipeNow, macroIn(scope)) : v.immediate ? exec({ ...v, immediate: false }, pipeNow, scope, {}, true) : { ...v, scope: v.scope ?? scope };
			const builtin = builtins[cmd.name];
			const named: Record<string, ArgValue> = {};
			for (const [k, v] of Object.entries(cmd.named)) named[k] = await expand(v);
			const call: CommandCall = {
				named,
				unnamed: cmd.unnamed === null ? cur : await expand(cmd.unnamed),
				pipe: cur,
				run: (c) => (typeof c === "string" ? Promise.resolve(c) : exec(c, pipeNow, scope, {}, false)),
			};
			const out = builtin ? await builtin(call, { cmd, scope, expand }) : await (env.commands[cmd.name] ?? missing(cmd.name))(call);
			if (out !== undefined) cur = out;
		}
		return cur;
	};

	/** 묶음 실행: 새 범위(만든 자리의 범위를 바깥으로), 선언된 인자 + 넘긴 인자. catchReturn: /run·바로 실행이면 /return을 여기서 받는다 */
	const exec = async (c: Closure, input: string, from: Scope, args: Record<string, ArgValue>, catchReturn: boolean): Promise<string> => {
		const scope = new Scope(c.scope ?? from);
		const [params, body] = closureParams(c.source);
		for (const [k, v] of Object.entries({ ...params, ...args })) scope.let(k, typeof v === "string" ? expandMacros(v, input, macroIn(scope)) : v);
		try {
			return await run(body, input, scope);
		} catch (e) {
			if (catchReturn && e instanceof Return) return e.value;
			throw e;
		}
	};

	const missing = (name: string): Handler => {
		if (env.unsupported?.includes(name)) throw new ScriptError(`/${name}은 지원하지 않음`);
		throw new ScriptError(`모르는 커맨드: /${name}`);
	};

	/** /if·/while의 left·right: 변수 이름이면 그 값 (범위 → 대화·전역), 아니면 글 그대로 */
	const operand = (v: ArgValue | undefined, scope: Scope): string => {
		const t = asText(v);
		if (t && /^[\p{L}\w-]+$/u.test(t)) {
			if (scope.has(t)) return asText(scope.get(t));
			const chat = env.variable?.(t);
			if (chat !== undefined) return chat;
		}
		return t;
	};

	/** 반복 본문 실행: /break면 그 값으로 끝 */
	const loop = async (body: ArgValue, input: string, scope: Scope, name: string, each: (i: number) => Promise<boolean>, setIndex?: (s: Scope, i: number) => void): Promise<string> => {
		if (typeof body === "string") throw new ScriptError(`/${name}: 반복할 묶음 {: … :}이 필요함`);
		let cur = input;
		for (let i = 0; await each(i); i++) {
			if (i >= loopLimit) throw new ScriptError(`/${name} 반복이 ${loopLimit}바퀴를 넘어 멈춤`);
			const inner = new Scope(body.scope ?? scope);
			setIndex?.(inner, i);
			try {
				cur = await exec({ ...body, scope: inner }, cur, inner, {}, false);
			} catch (e) {
				if (e instanceof Break) return e.value;
				throw e;
			}
		}
		return cur;
	};

	type Builtin = (call: CommandCall, ctx: { cmd: Command; scope: Scope; expand: (v: ArgValue) => Promise<ArgValue> }) => Promise<string | void> | string | void;

	/** 수학 커맨드의 수들: 공백으로 나눈 값 또는 JSON 목록, 변수 이름이면 그 값 */
	const numbers = (unnamed: ArgValue, scope: Scope, name: string): number[] => {
		const t = asText(unnamed).trim();
		let parts: string[];
		try {
			const v = JSON.parse(t) as unknown;
			parts = Array.isArray(v) ? v.map(String) : t.split(/\s+/);
		} catch {
			parts = t ? t.split(/\s+/) : [];
		}
		return parts.map((p) => {
			const n = num(operand(p, scope));
			if (n === null) throw new ScriptError(`/${name}: 숫자가 아님 (${p})`);
			return n;
		});
	};
	const math = (name: string, min: number, f: (xs: number[]) => number): Builtin => ({ unnamed }, { scope }) => {
		const xs = numbers(unnamed, scope, name);
		if (xs.length < min) throw new ScriptError(`/${name}: 수가 ${min}개 이상 필요함`);
		const r = f(xs);
		if (!Number.isFinite(r)) throw new ScriptError(`/${name}: 결과가 수가 아님`);
		return String(r);
	};

	const builtins: Record<string, Builtin> = {
		// 수학 (SillyTavern과 같은 이름)
		add: math("add", 1, (xs) => xs.reduce((a, b) => a + b, 0)),
		sub: math("sub", 1, (xs) => xs.slice(1).reduce((a, b) => a - b, xs[0])),
		mul: math("mul", 1, (xs) => xs.reduce((a, b) => a * b, 1)),
		div: math("div", 2, ([a, b]) => {
			if (b === 0) throw new ScriptError("/div: 0으로 나눌 수 없음");
			return a / b;
		}),
		mod: math("mod", 2, ([a, b]) => {
			if (b === 0) throw new ScriptError("/mod: 0으로 나눌 수 없음");
			return a % b;
		}),
		pow: math("pow", 2, ([a, b]) => a ** b),
		max: math("max", 1, (xs) => Math.max(...xs)),
		min: math("min", 1, (xs) => Math.min(...xs)),
		abs: math("abs", 1, ([a]) => Math.abs(a)),
		sqrt: math("sqrt", 1, ([a]) => Math.sqrt(a)),
		round: math("round", 1, ([a]) => Math.round(a)),
		sin: math("sin", 1, ([a]) => Math.sin(a)),
		cos: math("cos", 1, ([a]) => Math.cos(a)),
		log: math("log", 1, ([a]) => Math.log(a)),
		// /rand · /rand 10 · /rand from=1 to=6 round=floor
		rand: ({ named, unnamed }, { cmd }) => {
			const to = num(asText(named.to ?? (cmd.unnamed === null ? "" : unnamed)) || "1") ?? 1;
			const from = num(asText(named.from ?? "0")) ?? 0;
			const r = from + Math.random() * (to - from);
			const how = asText(named.round);
			return String(how === "round" ? Math.round(r) : how === "ceil" ? Math.ceil(r) : how === "floor" ? Math.floor(r) : r);
		},
		// 글자
		len: ({ unnamed }) => String(lengthOf(asText(unnamed))),
		trimstart: ({ unnamed }) => trimStart(asText(unnamed)),
		trimend: ({ unnamed }) => trimEnd(asText(unnamed)),
		abort: () => {
			throw new Abort();
		},
		break: ({ named, unnamed, pipe }, { cmd }) => {
			throw new Break(cmd.unnamed === null && !named.value ? pipe : asText(named.value ?? unnamed));
		},
		return: ({ unnamed, pipe }, { cmd }) => {
			throw new Return(cmd.unnamed === null ? pipe : asText(unnamed));
		},
		if: async ({ named, unnamed, pipe }, { scope }) => {
			const ok = compare(operand(named.left, scope), operand(named.right, scope), asText(named.rule) || "eq");
			const branch = ok ? unnamed : named.else;
			if (branch === undefined) return undefined;
			return typeof branch === "string" ? branch : exec(branch, pipe, scope, {}, false);
		},
		// /while left=… right=… rule=… {: … :} — 조건은 바퀴마다 다시 본다 (변수 이름이면 그 값, 매크로도 다시 바꿈)
		while: async ({ unnamed, pipe }, { cmd, scope, expand }) => {
			const again = async () => {
				const l = await expand(cmd.named.left ?? "");
				const r = await expand(cmd.named.right ?? "");
				const rule = asText(await expand(cmd.named.rule ?? "eq")) || "eq";
				return compare(operand(l, scope), operand(r, scope), rule);
			};
			return loop(unnamed, pipe, scope, "while", () => again());
		},
		// /times 5 {: … {{timesIndex}} … :}
		times: async ({ unnamed, pipe }, { scope }) => {
			let count: string;
			let body: ArgValue | undefined;
			if (typeof unnamed === "string") {
				const [n, v] = nameAndValue(unnamed);
				count = n;
				body = v && typeof v !== "string" ? { ...v, scope } : undefined;
			} else throw new ScriptError("/times: 횟수가 필요함 (/times 5 {: … :})");
			const n = num(count);
			if (n === null || n < 0) throw new ScriptError(`/times: 횟수가 숫자가 아님 (${count})`);
			if (!body) throw new ScriptError("/times: 반복할 묶음 {: … :}이 필요함");
			return loop(body, pipe, scope, "times", async (i) => i < n, (s, i) => s.let("timesIndex", String(i)));
		},
		// /run 묶음 | /run 변수이름 | /run 세트.버튼  (이름 있는 인자는 묶음 인자로)
		run: async ({ named: given, unnamed, pipe }, { scope }) => {
			let target: ArgValue | undefined = unnamed;
			const named = { ...given };
			if (typeof unnamed === "string") {
				// "이름 a=1 b=2" → 이름 + 묶음 인자 (SillyTavern: /run f a=7)
				const m = /^\s*("[^"]*"|\S+)/.exec(unnamed);
				const head = m ? m[1].replace(/^"|"$/g, "") : "";
				const [extra] = closureParams(m ? unnamed.slice(m[0].length) : "");
				Object.assign(named, extra);
				const name = head.trim();
				const v = scope.get(name);
				if (v !== undefined) target = v;
				else {
					const source = env.quickReply?.(name);
					if (source === undefined) throw new ScriptError(`/run: 실행할 것을 찾지 못함 (${name})`);
					target = { kind: "closure", source, scope: root };
				}
			}
			if (typeof target === "string") target = { kind: "closure", source: target, scope };
			try {
				return await exec(target, pipe, scope, named, true);
			} catch (e) {
				if (e instanceof Break) return e.value;
				throw e;
			}
		},
		// /let key=이름 값  ·  /let 이름 값  (이 범위에 새로)
		let: ({ named, unnamed }, { cmd, scope }) => {
			const [name, value] = named.key !== undefined ? [asText(named.key), cmd.unnamed === null ? "" : unnamed] : nameAndValue(asText(unnamed));
			if (!name) throw new ScriptError("/let: 변수 이름이 필요함");
			scope.let(name, value === undefined ? "" : typeof value === "string" || !value ? value : { ...value, scope: value.scope ?? scope });
		},
		// /var key=이름 값 (있는 범위에 바꿈) · /var 이름 (값 읽기) · index=로 목록·객체 자리
		var: ({ named, unnamed }, { cmd, scope }) => {
			let name: string;
			let value: ArgValue | undefined;
			if (named.key !== undefined) [name, value] = [asText(named.key), cmd.unnamed === null ? undefined : unnamed];
			else [name, value] = nameAndValue(asText(unnamed));
			if (!name) throw new ScriptError("/var: 변수 이름이 필요함");
			const index = named.index === undefined ? undefined : asText(named.index);
			if (value === undefined) {
				const cur = asText(scope.get(name));
				return index !== undefined ? (getIndexed(cur, index) ?? "") : cur;
			}
			scope.set(name, index !== undefined ? setIndexed(asText(scope.get(name)), index, asText(value)) : typeof value === "string" ? value : { ...value, scope: value.scope ?? scope });
		},
	};

	try {
		return { pipe: await run(src, pipe, root), aborted: false };
	} catch (e) {
		if (e instanceof Abort) return { pipe: "", aborted: true };
		if (e instanceof Return) return { pipe: e.value, aborted: false };
		if (e instanceof Break) return { pipe: e.value, aborted: false };
		throw e;
	}
}
