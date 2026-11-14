<script lang="ts">
	import { BatteryFull, ChevronLeft, Menu, Plus, Search, Send, Signal, Wifi } from "@lucide/svelte";
	import { Avatar } from "$shared/ui";

	const chats = [
		{ name: "Seraphina", last: "숲은 아직 위험해요. 조금 더 쉬어요.", time: "오후 10:42", unread: 2 },
		{ name: "Aria", last: "내일 성 앞에서 만나자!", time: "오후 7:15", unread: 0 },
		{ name: "Min", last: "사진", time: "월요일", unread: 1 },
		{ name: "카페 사장님", last: "오늘 휴무예요~", time: "11. 2.", unread: 0 },
	];
	let draft = $state("");
</script>

<!-- 폰: 실제 메신저 앱처럼. 데스크탑에서는 기기 모양 안에, 모바일에서는 화면 전체 -->
<div class="stage">
	<div class="device" aria-label="채팅 목록">
		<div class="status"><span>10:42</span><span class="icons"><Signal /><Wifi /><BatteryFull /></span></div>
		<div class="app-bar">
			<h2>채팅</h2>
			<span class="bar-icons"><Search /><Plus /></span>
		</div>
		<ul class="list">
			{#each chats as c, i (c.name)}
				<li class:on={i === 0}>
					<Avatar name={c.name} size={46} />
					<div class="main">
						<div class="row"><strong>{c.name}</strong><span class="time">{c.time}</span></div>
						<div class="row"><span class="last">{c.last}</span>{#if c.unread}<span class="badge">{c.unread}</span>{/if}</div>
					</div>
				</li>
			{/each}
		</ul>
		<div class="home-indicator"></div>
	</div>

	<div class="device room" aria-label="Seraphina와의 채팅방">
		<div class="status"><span>10:42</span><span class="icons"><Signal /><Wifi /><BatteryFull /></span></div>
		<div class="room-bar">
			<ChevronLeft />
			<strong>Seraphina</strong>
			<Menu />
		</div>
		<div class="msgs">
			<div class="date">2026년 11월 8일 일요일</div>
			<div class="them">
				<Avatar name="Seraphina" size={36} />
				<div class="stack">
					<span class="name">Seraphina</span>
					<div class="line"><div class="b">눈 떴어요?</div></div>
					<div class="line"><div class="b">어제 숲에서 쓰러져 있길래 데려왔어요</div><span class="t">오후 10:40</span></div>
				</div>
			</div>
			<div class="me">
				<div class="line"><span class="t"><span class="read">1</span>오후 10:41</span><div class="b">여기 어디예요??</div></div>
			</div>
			<div class="them">
				<Avatar name="Seraphina" size={36} />
				<div class="stack">
					<span class="name">Seraphina</span>
					<div class="line"><div class="photo"></div></div>
					<div class="line"><div class="b">제 오두막이에요. 조금 더 쉬어요.</div><span class="t">오후 10:42</span></div>
				</div>
			</div>
		</div>
		<form class="input" onsubmit={(e) => e.preventDefault()}>
			<button type="button" class="plus" aria-label="첨부"><Plus /></button>
			<input bind:value={draft} placeholder="메시지 입력" aria-label="메시지" />
			<button class="send" class:on={!!draft.trim()} aria-label="보내기"><Send /></button>
		</form>
		<div class="home-indicator"></div>
	</div>
</div>

<style>
	.stage {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-10);
		height: 100%;
		padding: var(--space-6);
		background:
			radial-gradient(circle at 30% 20%, var(--accent-soft), transparent 55%),
			var(--bg-sunken);
		overflow: auto;
	}
	.device {
		position: relative;
		display: flex;
		flex-direction: column;
		width: 360px;
		height: min(740px, 100%);
		flex: none;
		border-radius: 44px;
		border: 10px solid #1b1a18;
		background: #ffffff;
		color: #191919;
		box-shadow: var(--shadow-lg), inset 0 0 0 1px #000;
		overflow: hidden;
		--phone-accent: var(--accent);
		--phone-room: color-mix(in oklab, var(--accent) 14%, #eef1f4);
	}
	.status {
		display: flex;
		justify-content: space-between;
		padding: 10px 22px 6px;
		font-size: 13px;
		font-weight: 600;
	}
	.icons {
		display: flex;
		gap: 4px;
	}
	.icons :global(svg) {
		width: 14px;
		height: 14px;
	}
	.app-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 18px 12px;
	}
	.app-bar h2 {
		font-size: 22px;
	}
	.bar-icons {
		display: flex;
		gap: 16px;
	}
	.bar-icons :global(svg) {
		width: 22px;
		height: 22px;
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		flex: 1;
	}
	.list li {
		display: flex;
		gap: 12px;
		align-items: center;
		padding: 10px 18px;
	}
	.list li.on {
		background: #f2f2f2;
	}
	.list :global(.avatar) {
		border-radius: 16px !important;
	}
	.main {
		flex: 1;
		min-width: 0;
	}
	.row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 8px;
	}
	.time {
		font-size: 11px;
		color: #8a8a8a;
	}
	.last {
		font-size: 13px;
		color: #777;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.badge {
		min-width: 18px;
		height: 18px;
		padding: 0 5px;
		border-radius: 9px;
		background: #ff4d3b;
		color: white;
		font-size: 11px;
		font-weight: 700;
		display: grid;
		place-items: center;
	}
	.home-indicator {
		position: absolute;
		bottom: 8px;
		left: 50%;
		width: 120px;
		height: 4px;
		margin-left: -60px;
		border-radius: 2px;
		background: #111;
		opacity: 0.85;
	}

	/* ---- 채팅방 ---- */
	.room {
		background: var(--phone-room);
	}
	.room-bar {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 14px 10px;
	}
	.room-bar strong {
		flex: 1;
		font-size: 16px;
	}
	.room-bar :global(svg) {
		width: 22px;
		height: 22px;
	}
	.msgs {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 6px 12px;
		overflow: auto;
	}
	.date {
		align-self: center;
		padding: 3px 12px;
		border-radius: 12px;
		background: rgb(0 0 0 / 0.15);
		color: white;
		font-size: 11px;
	}
	.them {
		display: flex;
		gap: 8px;
		align-items: flex-start;
	}
	.them :global(.avatar) {
		border-radius: 14px !important;
	}
	.stack {
		display: flex;
		flex-direction: column;
		gap: 4px;
		max-width: 78%;
	}
	.name {
		font-size: 12px;
		color: #333;
	}
	.line {
		display: flex;
		align-items: flex-end;
		gap: 5px;
	}
	.me {
		align-self: flex-end;
		max-width: 78%;
	}
	.b {
		padding: 7px 10px;
		border-radius: 14px;
		background: white;
		font-size: 14px;
		line-height: 1.4;
	}
	.them .line:nth-child(2) .b {
		border-top-left-radius: 4px;
	}
	.me .b {
		background: var(--phone-accent);
		color: white;
		border-top-right-radius: 4px;
	}
	.t {
		flex: none;
		font-size: 10px;
		color: #555;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
	}
	.read {
		color: var(--phone-accent);
		font-weight: 700;
	}
	.photo {
		width: 180px;
		height: 130px;
		border-radius: 14px;
		background: linear-gradient(135deg, #e9d8b8, #9fb5a4);
	}
	.input {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 10px 24px;
		background: white;
	}
	.input input {
		flex: 1;
		height: 36px;
		padding: 0 14px;
		border: none;
		border-radius: 18px;
		background: #f1f1f1;
		outline: none;
		color: #191919;
	}
	.plus,
	.send {
		display: grid;
		place-items: center;
		width: 34px;
		height: 34px;
		border: none;
		border-radius: 50%;
		background: transparent;
		color: #666;
		cursor: pointer;
	}
	.send.on {
		background: var(--phone-accent);
		color: white;
	}
	.plus :global(svg),
	.send :global(svg) {
		width: 20px;
		height: 20px;
	}
	@media (max-width: 720px) {
		.stage {
			padding: 0;
			gap: 0;
		}
		.device:not(.room) {
			display: none;
		}
		.device {
			width: 100%;
			height: 100%;
			border: none;
			border-radius: 0;
		}
	}
</style>
