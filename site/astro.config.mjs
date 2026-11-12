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
