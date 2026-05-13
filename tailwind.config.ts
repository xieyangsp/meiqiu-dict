import type { Config } from 'tailwindcss';

export default {
  content: ['./index.html', './src-ui/**/*.{vue,ts,tsx}'],
  theme: {
    extend: {},
  },
  plugins: [],
} satisfies Config;
