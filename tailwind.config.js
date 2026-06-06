/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{svelte,ts}"],
  theme: {
    extend: {
      colors: {
        sypher: {
          bg: "#0b0f14",
          panel: "#121821",
          surface: "#18202b",
          border: "#263241",
          text: "#e6edf3",
          muted: "#8b9bab",
          accent: "#37d0a8",
          warn: "#f4b860",
        },
      },
      fontFamily: {
        sans: ["Inter", "ui-sans-serif", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "Cascadia Code", "ui-monospace", "monospace"],
      },
      borderRadius: {
        panel: "8px",
      },
    },
  },
  plugins: [],
};
