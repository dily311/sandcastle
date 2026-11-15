import { describe, expect, it } from "vitest";
import { splitHtmlBlocks, wholeHtmlBlock } from "./blocks";

describe("splitHtmlBlocks", () => {
	it("html 블록과 앞뒤 글을 나눈다", () => {
		const parts = splitHtmlBlocks('앞 글\n```html\n<div id="a">hi</div>\n```\n뒤 글');
		expect(parts).toEqual([
			{ kind: "text", text: "앞 글\n" },
			{ kind: "html", code: '<div id="a">hi</div>' },
			{ kind: "text", text: "\n뒤 글" },
		]);
	});

	it("언어 표시가 없어도 태그로 시작하면 HTML", () => {
		expect(splitHtmlBlocks("```\n<p>x</p>\n```")).toEqual([{ kind: "html", code: "<p>x</p>" }]);
	});

	it("다른 언어나 닫히지 않은 블록은 글로 둔다", () => {
		expect(splitHtmlBlocks("```js\nconsole.log(1)\n```")).toEqual([{ kind: "text", text: "```js\nconsole.log(1)\n```" }]);
		expect(splitHtmlBlocks("```html\n<div>스트리밍 중")).toEqual([{ kind: "text", text: "```html\n<div>스트리밍 중" }]);
	});

	it("글 전체가 블록 하나인지", () => {
		expect(wholeHtmlBlock("```html\n<b>x</b>\n```")).toBe("<b>x</b>");
		expect(wholeHtmlBlock("말\n```html\n<b>x</b>\n```")).toBeNull();
	});
});
