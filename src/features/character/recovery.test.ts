import { describe, expect, it } from "vitest";
import type { TaskInfo_Serialize } from "$ipc/bindings";
import { interruptedChats } from "./recovery";

const task = (id: string, taskType: string, status: string) => ({ id, taskType, status }) as unknown as TaskInfo_Serialize;

describe("interruptedChats", () => {
	it("멈춘 채팅 생성 작업만 고른다", () => {
		const tasks = [task("a", "chat_stream", "interrupted"), task("b", "image_gen", "interrupted"), task("c", "chat_stream", "failed")];
		expect(interruptedChats(tasks).map((t) => t.id)).toEqual(["a"]);
	});
});
