import "@testing-library/jest-dom/vitest";
import { setAppLocale } from "$lib/utils/i18n";

await setAppLocale("ko", { persistSetting: false });

