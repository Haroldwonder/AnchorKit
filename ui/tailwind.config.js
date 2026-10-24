/** @type {import('tailwindcss').Config} */
export default {
  content: [
    './components/**/*.{ts,tsx}',
    './src/**/*.{ts,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        'ak-bg': 'var(--ak-bg)',
        'ak-surface': 'var(--ak-surface)',
        'ak-border': 'var(--ak-border)',
        'ak-text': 'var(--ak-text)',
        'ak-text-muted': 'var(--ak-text-muted)',
        'pf-mint': 'var(--pf-mint)',
        'pf-mint-dim': 'var(--pf-mint-dim)',
        'pf-mint-border': 'var(--pf-mint-border)',
        'pf-mint-hover': 'var(--pf-mint-hover)',
        'pf-mint-glow': 'var(--pf-mint-glow)',
        'pf-mint-pulse-start': 'var(--pf-mint-pulse-start)',
        'pf-mint-pulse-end': 'var(--pf-mint-pulse-end)',
        'pf-bg': 'var(--pf-bg)',
        'pf-danger': 'var(--pf-danger)',
      },
      fontFamily: {
        mono: 'var(--pf-font-mono)',
        display: 'var(--pf-font-display)',
      },
      transition: {
        default: 'var(--pf-transition)',
      },
      spacing: {
        sm: 'var(--pf-spacing-sm)',
        lg: 'var(--pf-spacing-lg)',
        xl: 'var(--pf-spacing-xl)',
        '2xl': 'var(--pf-spacing-2xl)',
      },
      fontSize: {
        base: 'var(--pf-font-size-base)',
      },
    },
  },
  plugins: [],
};
