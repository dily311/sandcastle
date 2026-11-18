// 패널 화면: 지금 대화의 글자 수를 읽어 보여 준다. 읽기만 하므로 AI도 부르지 않는다 (요금 없음)
(async () => {
	const out = document.getElementById("out");
	const detail = document.getElementById("detail");
	const last = document.getElementById("last");
	try {
		const ctx = await sandcastle.app.context(); // 지금 화면 (대화 id 등)
		if (!ctx.conversationId) {
			out.textContent = "대화를 열면 셉니다";
			return;
		}
		const s = await sandcastle.settings.get();
		const limit = Math.min(200, Math.max(1, Math.floor(Number(s.recent) || 100)));
		const messages = await sandcastle.data.chats.messages(ctx.conversationId, { limit });
		const text = messages.map((m) => m.content).join("");
		const chars = s.countSpaces ? text.length : text.replace(/\s/g, "").length;
		out.textContent = chars.toLocaleString() + " 글자";
		detail.textContent = "최근 메시지 " + messages.length + "개" + (s.countSpaces ? " · 공백 포함" : " · 공백 뺌");
		const saved = await sandcastle.storage.get("last");
		if (saved) last.textContent = "명령으로 마지막에 센 것: " + saved.chars.toLocaleString() + " 글자 (" + new Date(saved.at).toLocaleString() + ")";
	} catch (e) {
		out.textContent = "읽지 못했습니다";
		detail.textContent = String(e && e.message ? e.message : e);
	}
})();
