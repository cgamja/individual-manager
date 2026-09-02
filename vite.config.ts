import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // 팝오버(index)·바탕화면 펭귄(pet)·핀볼 판(pinball)·볼링 공(ball)·비치발리볼
  // 코트와 공(volley-*)은 별도 엔트리다 — 상태·CSS를 섞지 않는다 (KTD8).
  // 펭귄 창 말고는 React를 쓰지 않는다: 판은 커서 CSS와 Esc 핸들러가 전부고,
  // 볼링 공은 SVG 하나와 포인터 셋, 비치발리볼의 둘은 SVG와 클래스 토글이
  // 전부라 그릴 트리가 없다.
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        pet: "pet.html",
        pinball: "pinball.html",
        ball: "ball.html",
        "volley-court": "volley-court.html",
        "volley-ball": "volley-ball.html",
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
