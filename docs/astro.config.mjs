// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// The site is published to GitHub Pages under /notiflow/, so `base` has to match the
// repository name or every internal link 404s on the deployed site while working locally.
export default defineConfig({
  site: 'https://jtprogru.github.io',
  base: '/notiflow',
  integrations: [
    starlight({
      title: 'notiflow',
      description:
        'Telegram notifier for CI and the terminal — a GitHub Action and a standalone CLI in one binary.',
      logo: { src: './src/assets/logo.svg', replacesTitle: false },
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/jtprogru/notiflow' },
      ],
      editLink: {
        baseUrl: 'https://github.com/jtprogru/notiflow/edit/main/docs/',
      },
      // English is the primary language. Russian pages that do not exist yet fall back to
      // their English originals rather than 404ing, which is why a partial locale is fine.
      defaultLocale: 'root',
      locales: {
        root: { label: 'English', lang: 'en' },
        ru: { label: 'Русский', lang: 'ru' },
      },
      sidebar: [
        {
          label: 'Start here',
          translations: { ru: 'Начало' },
          items: [
            { slug: 'getting-started' },
            { slug: 'install' },
          ],
        },
        {
          label: 'GitHub Action',
          items: [
            { slug: 'action/usage' },
            { slug: 'action/reference' },
            { slug: 'action/migration' },
          ],
        },
        {
          label: 'CLI',
          items: [
            { slug: 'cli/usage' },
            { slug: 'cli/configuration' },
            { slug: 'cli/reference' },
          ],
        },
        {
          label: 'Reference',
          translations: { ru: 'Справочник' },
          items: [
            { slug: 'templates' },
            { slug: 'parse-modes' },
            { slug: 'reliability' },
            { slug: 'recipes' },
          ],
        },
        {
          label: 'Project',
          translations: { ru: 'Проект' },
          items: [
            { slug: 'architecture' },
            { slug: 'contributing' },
          ],
        },
      ],
      lastUpdated: true,
      credits: false,
    }),
  ],
});
