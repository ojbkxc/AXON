/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        bg: {
          DEFAULT: "#0b0f14",
          soft: "#11161d",
          panel: "#161c26",
          hover: "#1c2430",
        },
        border: {
          DEFAULT: "#232b38",
          soft: "#1b222c",
        },
        accent: {
          DEFAULT: "#3b82f6",
          hover: "#2563eb",
        },
      },
      fontFamily: {
        sans: ["system-ui", "-apple-system", "Segoe UI", "Roboto", "sans-serif"],
        mono: ["ui-monospace", "SFMono-Regular", "Menlo", "monospace"],
      },
    },
  },
  plugins: [],
};
