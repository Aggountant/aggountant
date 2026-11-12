// Reads the user guide in place from docs/user/ for the site: entry ids and
// titles, the GitHub-relative links rewritten to site URLs, and the sidebar
// taken from the front pages. The Markdown itself is never copied or edited.
import { readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { glob } from 'astro/loaders';

const repoRoot = fileURLToPath(new URL('..', import.meta.url));
export const guideDir = path.join(repoRoot, 'docs/user');
const repoUrl = 'https://github.com/aggountant/aggountant';

// Finnish is the root locale, so docs/user/fi/tasks/close-month.md becomes
// /tasks/close-month/ and its English pair /en/tasks/close-month/. The front
// page (README.md) is /guide/, because / is the site's landing page.
function entryId(guidePath) {
  const [lang, ...rest] = guidePath.replace(/\.md$/, '').split('/');
  const page = rest.join('/') === 'README' ? 'guide' : rest.join('/');
  return lang === 'fi' ? page : `${lang}/${page}`;
}

function h1(filePath) {
  const match = readFileSync(filePath, 'utf-8').match(/^# (.+)$/m);
  if (!match) throw new Error(`${filePath}: the page has no H1 to use as its title`);
  return match[1].replace(/`/g, '').trim();
}

// The pages carry no frontmatter: Starlight's required title is the page's
// own H1, and guideMarkdown() drops that H1 from the body so it is not shown
// twice.
export function guideLoader() {
  const inner = glob({
    base: guideDir,
    pattern: ['fi/**/*.md', 'en/**/*.md'],
    generateId: ({ entry }) => entryId(entry),
  });
  return {
    ...inner,
    name: 'aggountant-guide-loader',
    load: (context) =>
      inner.load({
        ...context,
        parseData: (entry) =>
          context.parseData({ ...entry, data: { title: h1(entry.filePath), ...entry.data } }),
      }),
  };
}

function siteLink(fromFile, target) {
  if (/^[a-z][a-z0-9+.-]*:/i.test(target) || target.startsWith('#')) return undefined;
  const [file, anchor] = target.split('#', 2);
  const hash = anchor ? `#${anchor}` : '';
  const absolute = path.resolve(path.dirname(fromFile), decodeURI(file));
  const inGuide = path.relative(guideDir, absolute).split(path.sep).join('/');
  if (/^(fi|en)\/.+\.md$/.test(inGuide)) return `/${entryId(inGuide)}/${hash}`;
  // Anything else (practice material, CONTRIBUTING.md) is shown on GitHub.
  const inRepo = path.relative(repoRoot, absolute).split(path.sep).join('/');
  let kind = 'blob';
  try {
    if (statSync(absolute).isDirectory()) kind = 'tree';
  } catch {
    // A missing file is reported by scripts/check-user-guide.sh.
  }
  return `${repoUrl}/${kind}/main/${inRepo}${hash}`;
}

// Sätteri mdast plugin for the guide pages: drops the H1 (it is the title),
// rewrites relative .md links to site URLs, and tells starlight-links-validator
// the page's URL, which it would otherwise derive from src/content/docs/.
export function guideMarkdown({ fileURL }) {
  if (!fileURL) return undefined;
  const filePath = fileURLToPath(fileURL);
  const inGuide = path.relative(guideDir, filePath).split(path.sep).join('/');
  if (!/^(fi|en)\/.+\.md$/.test(inGuide)) return undefined;
  return {
    name: 'aggountant-guide',
    before(_root, ctx) {
      if (ctx.data.astro) ctx.data.astro.frontmatter.slug = entryId(inGuide);
    },
    heading(node, ctx) {
      if (node.depth === 1) ctx.removeNode(node);
    },
    link(node, ctx) {
      const url = siteLink(filePath, node.url);
      if (url) ctx.setProperty(node, 'url', url);
    },
  };
}

function sections(lang) {
  const result = [];
  for (const line of readFileSync(path.join(guideDir, lang, 'README.md'), 'utf-8').split('\n')) {
    const heading = line.match(/^## (.+)$/);
    if (heading) result.push({ label: heading[1].trim(), pages: [] });
    else if (result.length) {
      for (const [, target] of line.matchAll(/\]\(([^)#\s]+\.md)(?:#[^)]*)?\)/g)) {
        const page = path.posix.normalize(`${lang}/${target}`);
        const pages = result.at(-1).pages;
        if (page.startsWith(`${lang}/`) && !pages.includes(page)) pages.push(page);
      }
    }
  }
  return result;
}

// The sidebar is the Finnish front page: its sections in their calendar
// order, each with the pages it links to. Section names come from the front
// page of each language (scripts/check-user-guide.sh keeps their headings in
// step); page labels are the pages' own titles.
export function guideSidebar() {
  const fi = sections('fi');
  const en = sections('en');
  return [
    // The front page's own H1 ("aggountant — käyttäjän opas") is too long
    // for a sidebar entry.
    { slug: 'guide', label: 'Oppaan etusivu', translations: { en: 'Guide home' } },
    ...fi.map((section, i) => ({
      label: section.label,
      translations: en[i] ? { en: en[i].label } : {},
      items: section.pages.map((page) => ({ slug: entryId(page) })),
    })),
  ];
}
