import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // 팝오버(index)·바탕화면 펭귄(pet)·핀볼 판(pinball)·볼링 공(ball)·비치발리볼
  // 코트와 공(volley-*)은 별도 엔트리다 — 상태·CSS를 섞지 않는다 (KTD8).
  // 판은 커서 CSS와 Esc 핸들러가 전부고, 볼링 공은 SVG 하나와 포인터 셋,
  // 비치발리볼의 둘은 SVG와 클래스 토글이 전부라 React를 안 쓴다.
  //
  // **React를 쓰는 창은 둘이다** — 펭귄 창과 야차의 미녀 펭귄 창(`queen`).
  // 미녀는 `<Penguin>` 그림을 그대로 재사용해야 해서(바닐라로 새로 그리면
  // 펭귄 그림이 두 벌이 된다) 엔트리를 따로 뒀다.
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        pet: "pet.html",
        pinball: "pinball.html",
        ball: "ball.html",
        "volley-court": "volley-court.html",
        "volley-ball": "volley-ball.html",
        queen: "queen.html",
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
