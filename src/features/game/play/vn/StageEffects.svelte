<script lang="ts">
	// 화면 효과 층 (5-E-22): 무대 위, 대사창 아래. 누르기를 막지 않는다. 움직임 줄이기 설정이면 입자 효과는 그리지 않는다
	import { stageFx } from "./stageFx.svelte";

	const PARTICLES = 36;
	const parts = Array.from({ length: PARTICLES }, (_, i) => ({
		left: (i * 37) % 100,
		delay: ((i * 53) % 100) / 20,
		dur: 4 + ((i * 29) % 40) / 10,
		size: 0.6 + ((i * 17) % 10) / 10,
	}));
	const BURSTS = [
		{ x: 25, y: 30, hue: 340 },
		{ x: 70, y: 22, hue: 45 },
		{ x: 50, y: 40, hue: 190 },
	];
	const SPARKS = Array.from({ length: 12 }, (_, k) => k * 30);
</script>

<div class="fx" aria-hidden="true">
	{#if stageFx.ambient === "fog"}
		<div class="fog"></div>
	{:else if stageFx.ambient}
		<div class="particles" class:snow={stageFx.ambient === "snow"} class:petals={stageFx.ambient === "petals"} class:rain={stageFx.ambient === "rain"}>
			{#each parts as p, i (i)}
				<span style:left="{p.left}%" style:animation-delay="-{p.delay}s" style:animation-duration="{stageFx.ambient === 'rain' ? p.dur / 6 : p.dur}s" style:--s={p.size}></span>
			{/each}
		</div>
	{/if}
	{#if stageFx.shot.flash}
		{#key stageFx.shot.flash.key}<div class="flash" style:animation-duration="{stageFx.shot.flash.ms}ms"></div>{/key}
	{/if}
	{#if stageFx.shot.blackout}
		{#key stageFx.shot.blackout.key}<div class="blackout" style:animation-duration="{stageFx.shot.blackout.ms}ms"></div>{/key}
	{/if}
	{#if stageFx.shot.fade_in}
		{#key stageFx.shot.fade_in.key}<div class="fade-in" style:animation-duration="{stageFx.shot.fade_in.ms}ms"></div>{/key}
	{/if}
	{#if stageFx.shot.fireworks}
		{#key stageFx.shot.fireworks.key}
			<div class="fireworks" style:--d="{stageFx.shot.fireworks.ms}ms">
				{#each BURSTS as b, i (i)}
					<div class="burst" style:left="{b.x}%" style:top="{b.y}%" style:--h={b.hue} style:animation-delay="{i * 220}ms">
						{#each SPARKS as a (a)}<i style:--a="{a}deg"></i>{/each}
					</div>
				{/each}
			</div>
		{/key}
	{/if}
</div>

<style>
	.fx {
		position: absolute;
		inset: 0;
		z-index: var(--z-float);
		overflow: hidden;
		pointer-events: none;
	}
	.flash,
	.blackout,
	.fade-in {
		position: absolute;
		inset: 0;
		animation-fill-mode: forwards;
		animation-timing-function: ease-out;
	}
	.flash {
		background: #fff;
		animation-name: flash;
	}
	.blackout {
		background: #000;
		animation-name: blackout;
	}
	.fade-in {
		background: #000;
		animation-name: fade-in;
	}
	@keyframes flash {
		from {
			opacity: 0.9;
		}
		to {
			opacity: 0;
		}
	}
	@keyframes blackout {
		0% {
			opacity: 0;
		}
		20%,
		75% {
			opacity: 1;
		}
		100% {
			opacity: 0;
		}
	}
	@keyframes fade-in {
		from {
			opacity: 1;
		}
		to {
			opacity: 0;
		}
	}
	.fog {
		position: absolute;
		inset: -20%;
		background:
			radial-gradient(ellipse at 30% 60%, rgb(255 255 255 / 0.35), transparent 60%),
			radial-gradient(ellipse at 70% 40%, rgb(255 255 255 / 0.3), transparent 55%);
		animation: fog 18s ease-in-out infinite alternate;
	}
	@keyframes fog {
		from {
			transform: translateX(-6%);
		}
		to {
			transform: translateX(6%);
		}
	}
	.particles span {
		position: absolute;
		top: -5%;
		display: block;
		animation-name: fall;
		animation-timing-function: linear;
		animation-iteration-count: infinite;
	}
	.snow span {
		width: calc(6px * var(--s));
		height: calc(6px * var(--s));
		border-radius: 50%;
		background: rgb(255 255 255 / 0.9);
	}
	.petals span {
		width: calc(10px * var(--s));
		height: calc(7px * var(--s));
		border-radius: 70% 0 70% 0;
		background: #f6b7c9;
		animation-name: petal;
	}
	.rain span {
		width: 1px;
		height: calc(18px * var(--s));
		background: rgb(200 220 255 / 0.6);
	}
	@keyframes fall {
		to {
			transform: translate(-4vw, 110vh);
		}
	}
	@keyframes petal {
		50% {
			transform: translate(6vw, 55vh) rotate(180deg);
		}
		to {
			transform: translate(-3vw, 110vh) rotate(360deg);
		}
	}
	.burst {
		position: absolute;
		width: 0;
		height: 0;
	}
	.burst i {
		position: absolute;
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: hsl(var(--h) 90% 65%);
		box-shadow: 0 0 8px hsl(var(--h) 90% 65%);
		opacity: 0;
		animation: spark var(--d) ease-out both;
		animation-delay: inherit;
	}
	@keyframes spark {
		from {
			transform: rotate(var(--a)) translateY(0);
			opacity: 1;
		}
		to {
			transform: rotate(var(--a)) translateY(-90px);
			opacity: 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.particles,
		.fireworks {
			display: none;
		}
	}
</style>
