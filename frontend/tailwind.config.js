/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Custom colors for graph visualization
        'node-function': '#3b82f6',
        'node-class': '#8b5cf6',
        'node-file': '#10b981',
        'node-reference': '#6b7280',
        'edge-call': '#3b82f6',
        'edge-import': '#10b981',
        'edge-reference': '#6b7280',
        'severity-critical': '#dc2626',
        'severity-high': '#f97316',
        'severity-medium': '#eab308',
        'severity-low': '#3b82f6',
      },
    },
  },
  plugins: [],
}
