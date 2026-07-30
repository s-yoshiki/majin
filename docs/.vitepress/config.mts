import { defineConfig } from 'vitepress';

export default defineConfig({
  title: 'majin',
  description: 'A terminal in your browser and on your desktop',
  base: '/majin/',
  lang: 'en-US',
  head: [['meta', { name: 'theme-color', content: '#2e5fd0' }]],
  themeConfig: {
    logo: undefined,
    nav: [
      { text: 'Guide', link: '/' },
      { text: 'Reference', link: '/architecture' },
    ],
    sidebar: [
      {
        text: 'Guide',
        items: [
          { text: 'Overview', link: '/' },
          { text: 'Development', link: '/development' },
          { text: 'Deploying the web build', link: '/deployment' },
          { text: 'Desktop build', link: '/desktop' },
        ],
      },
      {
        text: 'Reference',
        items: [
          { text: 'Architecture', link: '/architecture' },
          { text: 'Wire protocol', link: '/protocol' },
          { text: 'Security', link: '/security' },
          { text: 'Configuration', link: '/configuration' },
        ],
      },
    ],
    socialLinks: [{ icon: 'github', link: 'https://github.com/s-yoshiki/majin' }],
    editLink: {
      pattern: 'https://github.com/s-yoshiki/majin/edit/main/docs/:path',
      text: 'Edit this page on GitHub',
    },
    search: {
      provider: 'local',
    },
    outline: {
      level: [2, 3],
    },
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'majin',
    },
  },
});
