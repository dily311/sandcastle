import MarkdownIt from "markdown-it";
import type { MarkdownIt as MarkdownItType } from "markdown-it";

// 1. 타입을 클래스 인스턴스 타입인 'MarkdownIt'으로 지정하고 세미콜론을 콜론으로 수정
let mdInstance: MarkdownItType | null = null;

// 2. 반환 타입 역시 인스턴스 타입인 'MarkdownIt'으로 지정
export function getMD(): MarkdownItType {
	if (!mdInstance) {
		mdInstance = new MarkdownIt();
	}
	return mdInstance;
}
