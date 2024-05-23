import colors from "tailwindcss/colors";

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{html,js,vue,ts}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        gray: colors.neutral,
      },
    },
  },
};
