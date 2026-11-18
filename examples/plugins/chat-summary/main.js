// 대화 요약 — AI를 부르는 예제.
// AI 호출(sandcastle.ai.generate)은 요금이 들 수 있어서 앱이 실행 주체를 직접 정한다:
//   · 사용자가 누른 명령 안에서 부르면 '사용자가 한 것'으로 바로 실행된다 (명령 하나에 3번까지, 60초 안)
//   · 패널이나 숨은 곳에서 부르면 사용자에게 확인 창이 뜨거나, 설정의 '자동 AI 호출'을 켜야 한다
// 그래서 이 플러그인은 명령에서만 AI를 부르고, 패널은 저장된 결과만 보여 준다.
// 호출은 실행 기록에 '플러그인 · 대화 요약'으로 남고 예산 한도(설정 > 실행 기록)가 그대로 적용된다.

const LENGTH = {
	"한 줄": "한 문장으로",
	짧게: "세 문장 이내로",
	자세히: "다섯 문장 정도로 자세히",
};

sandcastle.on("command", async ({ id, context }) => {
	if (id !== "summarize") return;
	const conversationId = context.conversationId;
	if (!conversationId) return sandcastle.warn("대화를 연 상태에서 눌러 주세요");

	const { length } = await sandcastle.settings.get();
	const messages = await sandcastle.data.chats.messages(conversationId, { limit: 40 });
	if (!messages.length) return sandcastle.warn("요약할 메시지가 없습니다");

	// 보내는 글은 20,000자까지. 길면 앞을 자른다
	const transcript = messages
		.map((m) => (m.name ? m.name + ": " : "") + m.content)
		.join("\n")
		.slice(-12000);
	const result = await sandcastle.ai.generate(transcript, {
		system: "다음 대화를 " + (LENGTH[length] ?? LENGTH["짧게"]) + " 한국어로 요약해. 요약만 쓰고 다른 말은 하지 마.",
		maxTokens: 300,
		temperature: 0.3,
	});
	await sandcastle.storage.set("summary:" + conversationId, { text: result.text, model: result.model, at: Date.now(), messages: messages.length });
	sandcastle.log("요약 완료", messages.length, "개 메시지 →", result.text.length, "자");
});
