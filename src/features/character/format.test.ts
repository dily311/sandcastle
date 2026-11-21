import { describe, expect, it } from "vitest";
import { preview, toBlocks } from "./format";

describe("toBlocks", () => {
	it("줄마다 문단을 만들고 빈 줄은 버린다", () => {
		const blocks = toBlocks("첫 줄\n\n  둘째 줄  ");
		expect(blocks).toEqual([
			{ kind: "p", parts: [{ kind: "text", text: "첫 줄" }] },
			{ kind: "p", parts: [{ kind: "text", text: "둘째 줄" }] },
		]);
	});

	it("*지문*과 \"대사\"를 나눈다", () => {
		const [block] = toBlocks('*웃는다* "안녕" 하고 말했다');
		expect(block).toEqual({
			kind: "p",
			parts: [
				{ kind: "action", text: "웃는다" },
				{ kind: "text", text: " " },
				{ kind: "speech", text: '"안녕"' },
				{ kind: "text", text: " 하고 말했다" },
			],
		});
	});

	it("<cg> 태그는 문단 사이 블록이 된다", () => {
		const blocks = toBlocks("앞<cg> smile </cg>뒤");
		expect(blocks.map((b) => b.kind)).toEqual(["p", "cg", "p"]);
		expect(blocks[1]).toEqual({ kind: "cg", key: "smile" });
	});
});

describe("마크다운 (5-E-1)", () => {
	it("####부터 ######까지 제목", () => {
		expect(toBlocks("#### 넷\n###### 여섯")).toEqual([
			{ kind: "h", level: 3, parts: [{ kind: "text", text: "넷" }] },
			{ kind: "h", level: 3, parts: [{ kind: "text", text: "여섯" }] },
		]);
	});

	it("* **굵게** 는 목록 + 굵게, *지문*은 그대로", () => {
		expect(toBlocks("* **내용**")).toEqual([{ kind: "li", parts: [{ kind: "strong", text: "내용" }] }]);
		expect(toBlocks("*웃는다*")[0]).toEqual({ kind: "p", parts: [{ kind: "action", text: "웃는다" }] });
	});

	it("코드 블록은 줄을 나누지 않고, 닫히지 않아도 끝까지", () => {
		expect(toBlocks("앞\n```js\nconst a = 1;\n\nb()\n```\n뒤")).toEqual([
			{ kind: "p", parts: [{ kind: "text", text: "앞" }] },
			{ kind: "code", lang: "js", text: "const a = 1;\n\nb()" },
			{ kind: "p", parts: [{ kind: "text", text: "뒤" }] },
		]);
		expect(toBlocks("```\n받는 중")).toEqual([{ kind: "code", lang: "", text: "받는 중" }]);
	});

	it("표", () => {
		const [t] = toBlocks("| 이름 | 나이 |\n|---|:--:|\n| 아리아 | **17** |\n| a\\|b | 2 |");
		expect(t).toEqual({
			kind: "table",
			head: [[{ kind: "text", text: "이름" }], [{ kind: "text", text: "나이" }]],
			rows: [
				[[{ kind: "text", text: "아리아" }], [{ kind: "strong", text: "17" }]],
				[[{ kind: "text", text: "a|b" }], [{ kind: "text", text: "2" }]],
			],
		});
	});

	it("구분줄이 없으면 표가 아님", () => {
		expect(toBlocks("| 그냥 | 글 |")[0].kind).toBe("p");
	});
});

describe("코드 블록 밖 HTML (5-E-9)", () => {
	it("태그로 시작하는 줄부터 닫힐 때까지 스크립트 없는 HTML 블록", () => {
		const b = toBlocks('*걷는다*\n<div class="st">\n<b>HP</b> 80\n</div>\n"좋아"', { safeHtml: true });
		expect(b.map((x) => x.kind)).toEqual(["p", "html", "p"]);
		expect(b[1]).toEqual({ kind: "html", code: '<div class="st">\n<b>HP</b> 80\n</div>', scripts: false });
	});

	it("```html 블록은 지금처럼 스크립트 허용, safeHtml이 없으면 글자로", () => {
		const b = toBlocks("```html\n<script>x()</script>\n```", { safeHtml: true });
		expect(b).toEqual([{ kind: "html", code: "<script>x()</script>" }]);
		expect(toBlocks("<div>안녕</div>").every((x) => x.kind !== "html")).toBe(true);
	});

	it("한 줄 안의 태그(<br>·<img>)도 그 줄만 블록", () => {
		const b = toBlocks("앞\n<img src=\"a.png\">\n뒤", { safeHtml: true });
		expect(b.map((x) => x.kind)).toEqual(["p", "html", "p"]);
	});
});

describe("preview", () => {
	it("태그·기호를 지우고 공백을 합친다", () => {
		expect(preview('*웃음*  "안녕"\n<cg>x</cg>')).toBe("웃음 안녕");
	});

	it("길면 자르고 말줄임표를 붙인다", () => {
		expect(preview("가".repeat(50), 10)).toBe(`${"가".repeat(10)}…`);
	});
});

describe("마크다운 줄과 이름 바꾸기", () => {
	it("제목·목록·인용·가로줄·굵게·코드", async () => {
		const { toBlocks } = await import("./format");
		const b = toBlocks("# 제목\n- 하나\n> 인용\n---\n**굵게** `코드`");
		expect(b.map((x) => x.kind)).toEqual(["h", "li", "quote", "hr", "p"]);
		expect(b[4]).toEqual({ kind: "p", parts: [{ kind: "strong", text: "굵게" }, { kind: "text", text: " " }, { kind: "code", text: "코드" }] });
	});

	it("메시지에서는 코드 블록 밖 HTML을 실행하지 않고, 개요(rawHtml)에서는 HTML 블록으로", async () => {
		const { toBlocks } = await import("./format");
		expect(toBlocks("<div>안녕</div>").every((x) => x.kind !== "html")).toBe(true);
		expect(toBlocks("<div>안녕</div>", { rawHtml: true })[0].kind).toBe("html");
	});

	it("{{char}}/{{user}}를 이름으로", async () => {
		const { fillNames } = await import("./format");
		expect(fillNames("{{char}}와 {{ User }}", { char: "엘리안", user: "나" })).toBe("엘리안와 나");
		expect(fillNames("{{user}}", {})).toBe("{{user}}");
	});
});
