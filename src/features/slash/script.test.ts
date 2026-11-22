import { describe, expect, it } from "vitest";
import { compare, parseCommand, parseScript, runScript, ScriptError, type Handler, type ScriptEnv } from "./script";

// 사용자가 빠른 답장에 쓰는 스크립트 (ADR-025)
const QUICK = `/input 빠르게 지시할 내용은? | /setvar key=user_topic {{pipe}} ||

/if left={{getvar::user_topic}} right="" rule=eq {:
/echo color=blue "생성을 취소하였습니다." |
/abort
:} ||

/gen 프롬프트 내용 : {{getvar::user_topic}} | /sendas name="{{char}}"`;

/** 가짜 앱: 입력 값, 변수, 남긴 일 */
function fakeApp(input: string) {
	const vars: Record<string, string> = {};
	const log: string[] = [];
	const text = (v: unknown) => (typeof v === "string" ? v : "");
	const commands: Record<string, Handler> = {
		input: () => input,
		setvar: ({ named, unnamed }) => {
			vars[text(named.key)] = text(unnamed);
		},
		getvar: ({ unnamed }) => vars[text(unnamed)] ?? "",
		echo: ({ named, unnamed }) => void log.push(`echo(${text(named.color)}): ${text(unnamed)}`),
		gen: ({ unnamed }) => `AI가 쓴 글 [${text(unnamed)}]`,
		sendas: ({ named, unnamed }) => void log.push(`sendas(${text(named.name)}): ${text(unnamed)}`),
	};
	const env: ScriptEnv = {
		commands,
		macro: (name, arg) => (name === "char" ? "세라피나" : name === "getvar" ? (vars[arg ?? ""] ?? "") : undefined),
		unsupported: ["fetch"],
	};
	return { env, vars, log };
}

describe("슬래시 스크립트: 읽기", () => {
	it("맨 바깥의 |로만 나눈다 (따옴표로 시작한 인자·묶음·\\| 안은 그대로)", () => {
		const cmds = parseScript('/a "x|y" | /b {: /c | /d :} | /e \\| f');
		expect(cmds.map((c) => [c.name, c.unnamed])).toEqual([
			["a", "x|y"],
			["b", { kind: "closure", source: " /c | /d " }],
			["e", "| f"],
		]);
	});
	it("글 중간의 따옴표는 글자 — 짝이 안 맞아도 다음 |에서 끝난다", () => {
		const cmds = parseScript('/gen 그녀가 "안녕 이라고 했다 | /sendas name="{{char}}"');
		expect(cmds.map((c) => c.name)).toEqual(["gen", "sendas"]);
		expect(cmds[0].unnamed).toBe('그녀가 "안녕 이라고 했다');
		expect(cmds[1].named.name).toBe("{{char}}");
	});
	it("|| 는 파이프를 끊고, 주석(// /# /* */)과 /로 시작하지 않는 조각은 건너뛴다", () => {
		const cmds = parseScript("/a || /# 메모 | /* 블록\n주석 */ /b | 그냥 글 | /c");
		expect(cmds.map((c) => [c.name, !!c.breakPipe])).toEqual([
			["a", false],
			["b", true],
			["c", false],
		]);
	});
	it("/:세트.버튼 → /run, {: :}() 바로 실행 묶음, 목록 값", () => {
		expect(parseCommand('/:"지시.빠른 지시"')).toMatchObject({ name: "run", unnamed: "지시.빠른 지시" });
		expect(parseCommand("/:세트.버튼")).toMatchObject({ name: "run", unnamed: "세트.버튼" });
		expect(parseCommand("/echo {: /getvar hp :}()")?.unnamed).toEqual({ kind: "closure", source: " /getvar hp ", immediate: true });
		expect(parseCommand('/buttons labels=["예","아니오"] 고르세요')).toMatchObject({ named: { labels: '["예","아니오"]' }, unnamed: "고르세요" });
	});
	it("이름 있는 인자·따옴표·묶음·이름 없는 인자", () => {
		expect(parseCommand('/if left=a right="" rule=eq {: /abort :}')).toEqual({
			name: "if",
			named: { left: "a", right: "", rule: "eq" },
			unnamed: { kind: "closure", source: " /abort " },
		});
		expect(parseCommand('/echo color=blue "따옴표 글"')).toEqual({ name: "echo", named: { color: "blue" }, unnamed: "따옴표 글" });
		expect(parseCommand("/gen 프롬프트 내용 : x")?.unnamed).toBe("프롬프트 내용 : x");
		expect(parseCommand("/trigger")?.unnamed).toBeNull();
	});
	it("빈 커맨드(||)와 // 주석은 건너뛰고, 닫히지 않은 묶음은 오류", () => {
		expect(parseScript("/a || // 메모 | /b").map((c) => c.name)).toEqual(["a", "b"]);
		expect(() => parseScript("/if {: /a")).toThrow(ScriptError);
	});
});

// 사용자가 준 실제 SillyTavern QR 파일 (src-tauri/tests/sillyTavern/qr)
const QR_FILES = import.meta.glob("../../../src-tauri/tests/sillyTavern/qr/*.json", { eager: true, import: "default" }) as Record<string, { message: string }>;
const qr = (name: string) => QR_FILES[`../../../src-tauri/tests/sillyTavern/qr/${name}`].message;

describe("실제 QR 파일 읽기 (5-E-20-6-1)", () => {
	it("분석: input → setvar → if(echo·abort) → gen → comment, gen 본문은 </System>까지", () => {
		const cmds = parseScript(qr("(분석)텍스트만.qr.json"));
		expect(cmds.map((c) => c.name)).toEqual(["input", "setvar", "if", "gen", "comment"]);
		expect(parseScript((cmds[2].unnamed as { source: string }).source).map((c) => c.name)).toEqual(["echo", "abort"]);
		expect(String(cmds[3].unnamed).endsWith("</System>")).toBe(true);
		expect(String(cmds[4].unnamed)).toContain("<details>");
		expect(cmds[3].breakPipe).toBe(true);
	});
	it("자동사냥: 본문 중간의 짝 안 맞는 따옴표에도 gen → sendas로 나뉜다", () => {
		const cmds = parseScript(qr("[생성] ❤️_🔥 자동사냥.qr.json"));
		expect(cmds.map((c) => c.name)).toEqual(["input", "setvar", "if", "gen", "sendas"]);
		expect(String(cmds[3].unnamed)).toContain("Omniscient third-person always");
		expect(cmds[4].named.name).toBe("{{char}}");
	});
	it("무물: 따옴표로 감싼 input 질문", () => {
		const cmds = parseScript(qr("무물🎤.qr.json"));
		expect(cmds.map((c) => c.name)).toEqual(["input", "gen", "sendas"]);
		expect(cmds[0].unnamed).toBe("Ask {{char}} anything");
	});
	it("뉴스: 이스케이프 안 된 | 뒤 조각은 건너뛴다 (SillyTavern과 같음)", () => {
		const cmds = parseScript(qr("📺뉴스.qr.json"));
		expect(cmds.map((c) => c.name)).toEqual(["setvar", "gen", "sendas", "flushvar"]);
		expect(cmds[0].named.key).toBe("news_instruction");
		expect(String(cmds[0].unnamed).endsWith("title: [Title]")).toBe(true);
		expect(cmds[2].named.name).toBe("{{groupNotMuted}}");
		expect(cmds[3].unnamed).toBe("news_instruction");
	});
});

describe("슬래시 스크립트: 실행", () => {
	it("사용자 예시: 입력이 있으면 AI 글을 캐릭터 말로 보낸다", async () => {
		const app = fakeApp("숲으로 가자");
		const r = await runScript(QUICK, app.env);
		expect(r.aborted).toBe(false);
		expect(app.vars.user_topic).toBe("숲으로 가자");
		expect(app.log).toEqual(["sendas(세라피나): AI가 쓴 글 [프롬프트 내용 : 숲으로 가자]"]);
	});
	it("사용자 예시: 입력이 비면 알리고 멈춘다 (AI를 부르지 않음)", async () => {
		const app = fakeApp("");
		const r = await runScript(QUICK, app.env);
		expect(r.aborted).toBe(true);
		expect(app.log).toEqual(["echo(blue): 생성을 취소하였습니다."]);
	});
	it("/if else=, 숫자 비교, 앞 결과 넘기기", async () => {
		const app = fakeApp("");
		app.vars.hp = "9";
		const r = await runScript('/getvar hp | /if left={{pipe}} right=10 rule=lt {: /echo "낮음" :} else={: /echo "높음" :}', app.env);
		expect(r.aborted).toBe(false);
		expect(app.log).toEqual(["echo(): 낮음"]);
		expect(compare("9", "10", "lt")).toBe(true);
		expect(compare("b", "abc", "in")).toBe(false);
		expect(compare("abc", "b", "in")).toBe(true);
	});
	it("모르는 커맨드·지원하지 않는 커맨드·단계 상한", async () => {
		const app = fakeApp("");
		await expect(runScript("/nope", app.env)).rejects.toThrow("모르는 커맨드: /nope");
		await expect(runScript("/fetch https://example.com", app.env)).rejects.toThrow("지원하지 않음");
		await expect(runScript("/echo a | /echo b | /echo c", { ...app.env, maxSteps: 2 })).rejects.toThrow("단계");
	});
});

describe("흐름·변수 (5-E-20-6-2)", () => {
	/** 가짜 앱 + 숫자 더하기 (/add 1 2 → 3) */
	const env = (extra: Partial<ScriptEnv> = {}) => {
		const app = fakeApp("");
		app.env.commands.add = ({ unnamed }) => String(String(unnamed).split(/\s+/).filter(Boolean).reduce((a, b) => a + Number(b), 0));
		return { app, env: { ...app.env, ...extra } };
	};
	it("/times와 {{timesIndex}}", async () => {
		const { app, env: e } = env();
		await runScript("/times 3 {: /echo 바퀴 {{timesIndex}} :}", e);
		expect(app.log).toEqual(["echo(): 바퀴 0", "echo(): 바퀴 1", "echo(): 바퀴 2"]);
	});
	it("/while은 조건을 바퀴마다 다시 본다 (left=변수 이름)", async () => {
		const { env: e } = env();
		const r = await runScript("/let i 0 | /while left=i right=3 rule=lt {: /add {{var::i}} 1 | /var key=i {{pipe}} :} | /var i", e);
		expect(r.pipe).toBe("3");
	});
	it("범위: 안쪽 /var는 바깥 변수를 바꾸고, 안쪽 /let은 밖에서 안 보인다", async () => {
		const { env: e } = env();
		expect((await runScript("/let x 1 | /run {: /var x 2 | /let y 5 :} | /var x", e)).pipe).toBe("2");
		expect((await runScript("/run {: /let y 5 :} | /var y", e)).pipe).toBe("");
	});
	it("/break는 반복을 그 값으로 끝내고, /return은 /run 묶음의 값", async () => {
		const { env: e } = env();
		expect((await runScript("/times 10 {: /if left={{timesIndex}} right=2 rule=eq {: /break 멈춤 :} :}", e)).pipe).toBe("멈춤");
		expect((await runScript("/let f {: a=1 /return {{var::a}}번 :} | /run f a=7", e)).pipe).toBe("7번");
		expect((await runScript("/let f {: a=1 /return {{arg::a}}번 :} | /:f", e)).pipe).toBe("1번");
	});
	it("{: :}() 바로 실행, 빠른 답장 부르기, 목록 index", async () => {
		const { app, env: e } = env({ quickReply: (n) => (n === "세트.버튼" ? "/echo 버튼이 불림" : undefined) });
		await runScript("/echo {: /return 안쪽 :}()", e);
		await runScript('/:"세트.버튼"', e);
		expect(app.log).toEqual(["echo(): 안쪽", "echo(): 버튼이 불림"]);
		expect((await runScript('/let list ["a","b"] | /var key=list index=1', e)).pipe).toBe("b");
		expect((await runScript('/let list ["a","b"] | /var key=list index=0 z | /var list', e)).pipe).toBe('["z","b"]');
		await expect(runScript("/run 없는것", e)).rejects.toThrow("찾지 못함");
	});
	it("반복 상한에 걸리면 오류로 멈춘다", async () => {
		const { env: e } = env({ maxLoop: 5 });
		await expect(runScript("/while left=1 right=1 {: /echo 또 :}", e)).rejects.toThrow("5바퀴");
		await expect(runScript("/times 6 {: /echo 또 :}", e)).rejects.toThrow("5바퀴");
	});
});
