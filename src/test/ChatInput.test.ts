// ChatInput.test.ts

import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";

import MessageForm from "$/screens/chat/MessageForm.svelte";

describe("ChatInput", () => {
  it("메시지를 입력하고 전송한다", async () => {
    const user = userEvent.setup();
    const onSend = vi.fn();

    render(MessageForm, {
      props: { onSend },
    });

    const input = screen.getByLabelText("메시지 입력");

    await user.type(input, "안녕하세요");
    await user.click(screen.getByRole("button", {
      name: "전송",
    }));

    expect(onSend).toHaveBeenCalledWith("안녕하세요");
  });

  it("빈 메시지는 전송하지 않는다", async () => {
    const onSend = vi.fn();

    render(MessageForm, {
      props: { onSend },
    });

    expect(onSend).not.toHaveBeenCalled();
  });
});