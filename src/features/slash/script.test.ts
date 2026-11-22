import { describe, expect, it } from "vitest";
import { compare, parseCommand, parseScript, runScript, ScriptError, splitPipes, type Handler, type ScriptEnv } from "./script";

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
		unsupported: ["while"],
	};
	return { env, vars, log };
}

describe("슬래시 스크립트: 읽기", () => {
	it("맨 바깥의 |로만 나눈다 (따옴표·묶음·\\| 안은 그대로)", () => {
		expect(splitPipes('/a "x|y" | /b {: /c | /d :} | /e \\| f')).toEqual(['/a "x|y" ', " /b {: /c | /d :} ", " /e \\| f"]);
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
	it("빈 커맨드(||)와 // 주석은 건너뛰고, /로 시작하지 않으면 오류", () => {
		expect(parseScript("/a || // 메모 | /b").map((c) => c.name)).toEqual(["a", "b"]);
		expect(() => parseScript("/a | 그냥 글")).toThrow(ScriptError);
		expect(() => parseScript("/if {: /a")).toThrow(ScriptError);
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
		await expect(runScript("/while left=1 {: /echo x :}", app.env)).rejects.toThrow("지원하지 않음");
		await expect(runScript("/echo a | /echo b | /echo c", { ...app.env, maxSteps: 2 })).rejects.toThrow("단계");
	});
});
