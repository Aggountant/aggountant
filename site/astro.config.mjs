// @ts-check
import { defineConfig } from 'astro/config';
import { satteri } from '@astrojs/markdown-satteri';
import starlight from '@astrojs/starlight';
import starlightLinksValidator from 'starlight-links-validator';
import { guideDir, guideMarkdown, guideSidebar } from './guide.mjs';

export default defineConfig({
  site: 'https://aggountant.dev',
  markdown: {
    processor: satteri({ mdastPlugins: [guideMarkdown] }),
  },
  integrations: [
    starlight({
      title: 'aggountant',
      defaultLocale: 'root',
      locales: {
        root: { label: 'Suomi', lang: 'fi' },
        en: { label: 'English', lang: 'en' },
      },
      // The Pääkirja (ledger) identity: theme, logo and light-first paper.
      customCss: ['./src/styles/ledger.css'],
      components: {
        SiteTitle: './src/components/SiteTitle.astro',
        ThemeProvider: './src/components/ThemeProvider.astro',
      },
      expressiveCode: {
        themes: ['github-dark-dimmed', 'github-light'],
        styleOverrides: {
          borderRadius: '0',
          borderColor: ({ theme }) => (theme.type === 'dark' ? '#34404a' : '#d8d0bb'),
          codeBackground: ({ theme }) => (theme.type === 'dark' ? '#11161a' : '#fffdf7'),
          codeFontFamily:
            "ui-monospace, 'SF Mono', 'Cascadia Mono', 'DejaVu Sans Mono', Menlo, Consolas, monospace",
          frames: {
            shadowColor: 'transparent',
            editorTabBarBackground: ({ theme }) => (theme.type === 'dark' ? '#1c242a' : '#ece6d6'),
            terminalTitlebarBackground: ({ theme }) =>
              theme.type === 'dark' ? '#1c242a' : '#ece6d6',
            terminalBackground: ({ theme }) => (theme.type === 'dark' ? '#11161a' : '#fffdf7'),
            terminalTitlebarBorderBottomColor: ({ theme }) =>
              theme.type === 'dark' ? '#34404a' : '#d8d0bb',
          },
        },
      },
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/aggountant/aggountant' },
      ],
      // Starlight's heading anchors and asides for the guide outside src/.
      markdown: { processedDirs: [guideDir] },
      sidebar: guideSidebar(),
      plugins: [starlightLinksValidator()],
    }),
  ],
});
