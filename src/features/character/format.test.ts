import { describe, expect, it } from "vitest";
import { fillNames, preview, toBlocks, type Block } from "./format";
import { renderUiHtml } from "$shared/html/markdown";

/** 마크다운 블록의 HTML (하나일 때) */
const md = (b: Block[]) => {
	const m = b.filter((x) => x.kind === "md");
	return m.map((x) => (x as { html: string }).html).join("|");
};

describe("GFM 렌더링 (5-E-27)", () => {
	it("줄마다 줄바꿈, 빈 줄은 문단", () => {
		expect(md(toBlocks("첫 줄\n둘째 줄\n\n셋째"))).toBe("<p>첫 줄<br>둘째 줄</p>\n<p>셋째</p>\n");
	});
	it("*지문* 기울임, \"대사\" 강조, **굵게**, ~~취소선~~, `코드`", () => {
		const h = md(toBlocks('*웃는다* "안녕" **굵게** ~~지움~~ `x` _밑줄 기울임_'));
		expect(h).toContain("<em>웃는다</em>");
		expect(h).toContain('<q>"안녕"</q>');
		expect(h).toContain("<strong>굵게</strong>");
		expect(h).toContain("<del>지움</del>");
		expect(h).toContain("<code>x</code>");
		expect(h).toContain("<em>밑줄 기울임</em>");
	});
	it("번호 목록·중첩 목록·작업 목록", () => {
		const h = md(toBlocks("1. 하나\n2. 둘\n   - 안쪽\n\n- [x] 끝\n- [ ] 남음"));
		expect(h).toMatch(/<ol>[\s\S]*<li>하나<\/li>[\s\S]*<ul>[\s\S]*<li>안쪽<\/li>/);
		expect(h).toContain('<span class="task">☑</span>');
		expect(h).toContain('<span class="task">☐</span>');
	});
	it("표(정렬)·제목·인용·가로줄·코드 블록", () => {
		const h = md(toBlocks("# 제목\n> 인용\n\n---\n\n| 이름 | 나이 |\n|---|:--:|\n| 아리아 | **17** |\n\n```js\nconst a = 1;\n```"));
		expect(h).toContain("<h1>제목</h1>");
		expect(h).toContain("<blockquote>");
		expect(h).toContain("<hr>");
		expect(h).toMatch(/<th>이름<\/th>[\s\S]*<th align="center">나이<\/th>/);
		expect(h).toContain('<td align="center"><strong>17</strong></td>');
		expect(h).toContain('<pre><code class="language-js">const a = 1;');
	});
	it("링크는 글자로(앱 창이 넘어가지 않게), javascript: 링크·그림 막힘", () => {
		const h = md(toBlocks("[숲](https://example.com) [x](javascript:alert(1)) ![a](javascript:x) https://auto.link"));
		expect(h).toContain('<span title="https://example.com" class="html-link">숲</span>');
		expect(h).not.toContain("javascript");
		expect(h).toContain("auto.link");
	});
	it("<cg>는 문단 사이 블록", () => {
		const b = toBlocks("앞<cg> smile </cg>뒤");
		expect(b.map((x) => x.kind)).toEqual(["md", "cg", "md"]);
		expect(b[1]).toEqual({ kind: "cg", key: "smile" });
	});
	it("코드 블록 안의 \"따옴표\"는 대사로 바꾸지 않는다", () => {
		expect(md(toBlocks('```\nsay("hi")\n```'))).not.toContain("<q>");
	});
});

describe("HTML (5-E-9·5-E-27)", () => {
	it("HTML 플러그인이 꺼져 있으면 태그는 글자로", () => {
		const h = md(toBlocks("<div>안녕</div> 그리고 <b>굵게</b>"));
		expect(h).toContain("&lt;div&gt;안녕&lt;/div&gt;");
		expect(h).not.toContain("<b>");
		expect(md(toBlocks("<details>\n- HP 80\n</details>"))).toBe("<p>&lt;details&gt;<br>- HP 80<br>&lt;/details&gt;</p>\n");
	});
	it("켜져 있으면 줄 안 태그는 걸러서 그리고, 태그로 시작하는 줄은 UI 블록", () => {
		const b = toBlocks('*걷는다* 그녀는 <b onclick="x()">웃었다</b>\n<div class="st">\n<b>HP</b> 80\n</div>\n"좋아"', { safeHtml: true });
		expect(b.map((x) => x.kind)).toEqual(["md", "html", "md"]);
		expect((b[0] as { html: string }).html).toContain("<b>웃었다</b>");
		expect((b[0] as { html: string }).html).not.toContain("onclick");
		expect(b[1]).toEqual({ kind: "html", code: '<div class="st">\n<b>HP</b> 80\n</div>', scripts: false });
	});
	it("```html 블록은 지금처럼 스크립트 허용 (iframe)", () => {
		expect(toBlocks("```html\n<script>x()</script>\n```", { safeHtml: true })).toEqual([{ kind: "html", code: "<script>x()</script>" }]);
	});
	it("UI HTML 안의 글도 마크다운으로 (한 줄은 꾸밈, 여러 줄은 목록·줄바꿈), 스크립트는 막힘", () => {
		const h = renderUiHtml('<details open><summary>**상태** 창</summary>\n- HP 80\n- MP *20*\n<script>alert(1)</script></details>');
		expect(h).toContain("<summary><strong>상태</strong> 창</summary>");
		expect(h).toMatch(/<ul>[\s\S]*<li>HP 80<\/li>[\s\S]*<li>MP <em>20<\/em><\/li>/);
		expect(h).not.toContain("script");
	});
	it("개요(rawHtml)에서는 글 전체를 HTML 블록으로", () => {
		expect(toBlocks("<div>안녕</div>", { rawHtml: true })[0].kind).toBe("html");
	});
});

describe("다인봇 말한 사람 (5-F-2)", () => {
	it("`이름: 대사` 줄은 말한 사람(그림·이름), 켜지 않으면 그대로", () => {
		const h = md(toBlocks('유나: "안녕" *웃는다*\n미나：응\n그냥 지문', { speakers: true, avatars: { 유나: "asset://yuna.png" } }));
		expect(h).toContain('<p class="say"><span class="who sc-speaker"><img class="av" src="asset://yuna.png"><b>유나</b></span><q>"안녕"</q> <em>웃는다</em></p>');
		expect(h).toContain('<span class="who sc-speaker"><span class="av">미</span><b>미나</b></span>응');
		expect(md(toBlocks("유나: 안녕"))).not.toContain("say");
	});
	it("지문·대사로 시작하거나 이름이 길면 말한 사람이 아님", () => {
		expect(md(toBlocks("*그녀가 말했다: 좋아*", { speakers: true }))).not.toContain("say");
		expect(md(toBlocks("이것은 아주 아주 아주 아주 긴 문장의 앞부분: 뒤", { speakers: true }))).not.toContain("say");
	});
});

describe("preview·이름 바꾸기", () => {
	it("태그·기호를 지우고 공백을 합친다", () => {
		expect(preview('*웃음*  "안녕"\n<cg>x</cg>')).toBe("웃음 안녕");
	});
	it("길면 자르고 말줄임표를 붙인다", () => {
		expect(preview("가".repeat(50), 10)).toBe(`${"가".repeat(10)}…`);
	});
	it("{{char}}/{{user}}를 이름으로", () => {
		expect(fillNames("{{char}}와 {{ User }}", { char: "엘리안", user: "나" })).toBe("엘리안와 나");
		expect(fillNames("{{user}}", {})).toBe("{{user}}");
	});
});
