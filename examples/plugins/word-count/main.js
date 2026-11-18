// 글자 수 — 메인 스크립트 (보이지 않는 곳에서 돈다. 화면은 ui/panel.html)
// 보여 주는 것: 명령 받기(sandcastle.on("command")), 앱 정보·지금 화면(app.info / app.context),
// 읽기 권한(data.chats), 설정 읽기(settings.get), 자기 저장소(storage)

/** 지금 보고 있는 대화의 글자 수를 센다 (공백 포함 여부와 몇 개까지는 설정에서) */
async function count(conversationId) {
	const s = await sandcastle.settings.get();
	const limit = Math.min(200, Math.max(1, Math.floor(Number(s.recent) || 100)));
	const messages = await sandcastle.data.chats.messages(conversationId, { limit });
	const text = messages.map((m) => m.content).join("");
	const chars = s.countSpaces ? text.length : text.replace(/\s/g, "").length;
	return { chars, messages: messages.length };
}

// 사용자가 타이틀바의 플러그인 명령 메뉴에서 누르면 불린다. 플러그인이 스스로 부를 수는 없다
sandcastle.on("command", async ({ id, context }) => {
	if (id !== "count") return;
	if (!context.conversationId) return sandcastle.warn("대화를 연 상태에서 눌러 주세요");
	const result = await count(context.conversationId);
	await sandcastle.storage.set("last", { ...result, conversationId: context.conversationId, at: Date.now() });
	sandcastle.log("센 결과", result.chars, "글자 /", result.messages, "개");
});

// 시작했다는 표시 (설정 > 커뮤니티 플러그인의 '기록 보기'에서 볼 수 있다)
sandcastle.app.info().then((info) => {
	sandcastle.log("시작", info.pluginId, "v" + info.version, "앱", info.appVersion);
	return sandcastle.storage.set("started", { at: Date.now(), appVersion: info.appVersion });
});
