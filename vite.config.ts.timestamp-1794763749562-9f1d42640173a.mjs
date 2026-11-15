// vite.config.ts
import { defineConfig } from "file:///workspace/node_modules/vite/dist/node/index.js";
import process from "node:process";
var host = process.env.TAURI_DEV_HOST;
var vite_config_default = defineConfig(() => ({
  test: {
    include: ["src/**/*.test.ts"],
    environmentMatchGlobs: [["src/wallpaper-ui.test.ts", "jsdom"]]
  },
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        scrollHarness: "scroll-harness.html",
        hebrewHarness: "hebrew-harness.html",
        gapHarness: "gap-harness.html",
        wallpaperHarness: "wallpaper-harness.html"
      }
    }
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
    hmr: host ? {
      protocol: "ws",
      host,
      port: 1421
    } : void 0,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"]
    }
  }
}));
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcudHMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCIvd29ya3NwYWNlXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ZpbGVuYW1lID0gXCIvd29ya3NwYWNlL3ZpdGUuY29uZmlnLnRzXCI7Y29uc3QgX192aXRlX2luamVjdGVkX29yaWdpbmFsX2ltcG9ydF9tZXRhX3VybCA9IFwiZmlsZTovLy93b3Jrc3BhY2Uvdml0ZS5jb25maWcudHNcIjsvLy8gPHJlZmVyZW5jZSB0eXBlcz1cInZpdGVzdC9jb25maWdcIiAvPlxuaW1wb3J0IHsgZGVmaW5lQ29uZmlnIH0gZnJvbSBcInZpdGVcIjtcbi8vIEB0cy1leHBlY3QtZXJyb3IgdHlwZSBlcnJvciB3aXRob3V0IEB0eXBlcy9ub2RlIHBhY2thZ2VcbmltcG9ydCBwcm9jZXNzIGZyb20gXCJub2RlOnByb2Nlc3NcIjtcbmNvbnN0IGhvc3QgPSBwcm9jZXNzLmVudi5UQVVSSV9ERVZfSE9TVDtcblxuLy8gaHR0cHM6Ly92aXRlLmRldi9jb25maWcvXG5leHBvcnQgZGVmYXVsdCBkZWZpbmVDb25maWcoKCkgPT4gKHtcbiAgdGVzdDoge1xuICAgIGluY2x1ZGU6IFtcInNyYy8qKi8qLnRlc3QudHNcIl0sXG4gICAgZW52aXJvbm1lbnRNYXRjaEdsb2JzOiBbW1wic3JjL3dhbGxwYXBlci11aS50ZXN0LnRzXCIsIFwianNkb21cIl1dLFxuICB9LFxuXG4gIGJ1aWxkOiB7XG4gICAgcm9sbHVwT3B0aW9uczoge1xuICAgICAgaW5wdXQ6IHtcbiAgICAgICAgbWFpbjogXCJpbmRleC5odG1sXCIsXG4gICAgICAgIHNjcm9sbEhhcm5lc3M6IFwic2Nyb2xsLWhhcm5lc3MuaHRtbFwiLFxuICAgICAgICBoZWJyZXdIYXJuZXNzOiBcImhlYnJldy1oYXJuZXNzLmh0bWxcIixcbiAgICAgICAgZ2FwSGFybmVzczogXCJnYXAtaGFybmVzcy5odG1sXCIsXG4gICAgICAgIHdhbGxwYXBlckhhcm5lc3M6IFwid2FsbHBhcGVyLWhhcm5lc3MuaHRtbFwiLFxuICAgICAgfSxcbiAgICB9LFxuICB9LFxuXG4gIC8vIFZpdGUgb3B0aW9ucyB0YWlsb3JlZCBmb3IgVGF1cmkgZGV2ZWxvcG1lbnQgYW5kIG9ubHkgYXBwbGllZCBpbiBgdGF1cmkgZGV2YCBvciBgdGF1cmkgYnVpbGRgXG4gIC8vXG4gIC8vIDEuIHByZXZlbnQgVml0ZSBmcm9tIG9ic2N1cmluZyBydXN0IGVycm9yc1xuICBjbGVhclNjcmVlbjogZmFsc2UsXG4gIC8vIDIuIHRhdXJpIGV4cGVjdHMgYSBmaXhlZCBwb3J0LCBmYWlsIGlmIHRoYXQgcG9ydCBpcyBub3QgYXZhaWxhYmxlXG4gIHNlcnZlcjoge1xuICAgIHBvcnQ6IDE0MjAsXG4gICAgc3RyaWN0UG9ydDogdHJ1ZSxcbiAgICBob3N0OiBob3N0IHx8IGZhbHNlLFxuICAgIGhtcjogaG9zdFxuICAgICAgPyB7XG4gICAgICAgICAgcHJvdG9jb2w6IFwid3NcIixcbiAgICAgICAgICBob3N0LFxuICAgICAgICAgIHBvcnQ6IDE0MjEsXG4gICAgICAgIH1cbiAgICAgIDogdW5kZWZpbmVkLFxuICAgIHdhdGNoOiB7XG4gICAgICAvLyAzLiB0ZWxsIFZpdGUgdG8gaWdub3JlIHdhdGNoaW5nIGBzcmMtdGF1cmlgXG4gICAgICBpZ25vcmVkOiBbXCIqKi9zcmMtdGF1cmkvKipcIl0sXG4gICAgfSxcbiAgfSxcbn0pKTtcbiJdLAogICJtYXBwaW5ncyI6ICI7QUFDQSxTQUFTLG9CQUFvQjtBQUU3QixPQUFPLGFBQWE7QUFDcEIsSUFBTSxPQUFPLFFBQVEsSUFBSTtBQUd6QixJQUFPLHNCQUFRLGFBQWEsT0FBTztBQUFBLEVBQ2pDLE1BQU07QUFBQSxJQUNKLFNBQVMsQ0FBQyxrQkFBa0I7QUFBQSxJQUM1Qix1QkFBdUIsQ0FBQyxDQUFDLDRCQUE0QixPQUFPLENBQUM7QUFBQSxFQUMvRDtBQUFBLEVBRUEsT0FBTztBQUFBLElBQ0wsZUFBZTtBQUFBLE1BQ2IsT0FBTztBQUFBLFFBQ0wsTUFBTTtBQUFBLFFBQ04sZUFBZTtBQUFBLFFBQ2YsZUFBZTtBQUFBLFFBQ2YsWUFBWTtBQUFBLFFBQ1osa0JBQWtCO0FBQUEsTUFDcEI7QUFBQSxJQUNGO0FBQUEsRUFDRjtBQUFBO0FBQUE7QUFBQTtBQUFBLEVBS0EsYUFBYTtBQUFBO0FBQUEsRUFFYixRQUFRO0FBQUEsSUFDTixNQUFNO0FBQUEsSUFDTixZQUFZO0FBQUEsSUFDWixNQUFNLFFBQVE7QUFBQSxJQUNkLEtBQUssT0FDRDtBQUFBLE1BQ0UsVUFBVTtBQUFBLE1BQ1Y7QUFBQSxNQUNBLE1BQU07QUFBQSxJQUNSLElBQ0E7QUFBQSxJQUNKLE9BQU87QUFBQTtBQUFBLE1BRUwsU0FBUyxDQUFDLGlCQUFpQjtBQUFBLElBQzdCO0FBQUEsRUFDRjtBQUNGLEVBQUU7IiwKICAibmFtZXMiOiBbXQp9Cg==
