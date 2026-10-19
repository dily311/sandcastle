/*! *****************************************************************************
가독성을 위해 색상을 조절하는 하무 모음집
***************************************************************************** */

interface ColorPalette {
	accentColor: string;
	onAccentTextColor: string;
	bgLightColor: string;
	textColor: string;
}

/** accentColor에 맞춰 색을 반환하는 함수 */
export function generateColorPalette(isDark: boolean, accentHex: string): ColorPalette {
	const { h, s, l } = hexToHSL(accentHex);

	if (isDark) {
		// 다크모드에서는 채도를 기본적으로 20~30% 수준으로 조절
		const mutedSaturation = Math.min(s, 25);
		return {
			accentColor: `hsl(${h}, ${Math.min(s, 60)}%, 60%)`,
			onAccentTextColor: getContrastYIQ(accentHex) ? "#fff" : "#000",
			textColor: `hsl(${h}, ${mutedSaturation}%, 90%)`,
			bgLightColor: `hsl(${h}, ${mutedSaturation}%, 10%)`,
		};
	}

	return {
		accentColor: accentHex,
		onAccentTextColor: getContrastYIQ(accentHex) ? "#000" : "#fff",
		textColor: `hsl(${h}, ${Math.min(s, 80)}%, 20%)`,
		bgLightColor: `hsl(${h}, ${Math.min(s, 60)}%, 94%)`,
	};
}

/** HEX -> HSL 변환 함수 */
function hexToHSL(hex: string): { h: number; s: number; l: number } {
	let r = parseInt(hex.slice(1, 3), 16) / 255;
	let g = parseInt(hex.slice(3, 5), 16) / 255;
	let b = parseInt(hex.slice(5, 7), 16) / 255;

	let max = Math.max(r, g, b),
		min = Math.min(r, g, b);
	let h,
		s,
		l = (max + min) / 2;

	if (max === min) {
		h = s = 0; // 무채색
	} else {
		let d = max - min;
		s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
		switch (max) {
			case r:
				h = (g - b) / d + (g < b ? 6 : 0);
				break;
			case g:
				h = (b - r) / d + 2;
				break;
			case b:
				h = (r - g) / d + 4;
				break;
			default:
				h = 0;
		}
		h /= 6;
	}
	return { h: Math.round(h * 360), s: Math.round(s * 100), l: Math.round(l * 100) };
}

function getContrastYIQ(hex: string): boolean {
	let r = parseInt(hex.slice(1, 3), 16);
	let g = parseInt(hex.slice(3, 5), 16);
	let b = parseInt(hex.slice(5, 7), 16);
	let yiq = (r * 299 + g * 587 + b * 114) / 1000;
	return yiq >= 128;
}
