import { defineConfig } from 'vitepress';

export default defineConfig({
  title: 'majin - 魔人🧞',
  description: 'A terminal in your browser and on your desktop',
  base: '/majin/',
  locales: {
    root: { label: 'English', lang: 'en-US' },
    ja: { label: '日本語', lang: 'ja-JP' },
  },
  head: [['meta', { name: 'theme-color', content: '#2e5fd0' }]],
  themeConfig: {
    logo: undefined,
    locales: {
      root: {
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
        editLink: {
          pattern: 'https://github.com/s-yoshiki/majin/edit/main/docs/:path',
          text: 'Edit this page on GitHub',
        },
        footer: {
          message: 'Released under the MIT License.',
          copyright: 'majin',
        },
      },
      ja: {
        nav: [
          { text: 'ガイド', link: '/ja/' },
          { text: 'リファレンス', link: '/ja/architecture' },
        ],
        sidebar: [
          {
            text: 'ガイド',
            items: [
              { text: '概要', link: '/ja/' },
              { text: '開発', link: '/ja/development' },
              { text: 'Web版のデプロイ', link: '/ja/deployment' },
              { text: 'デスクトップ版', link: '/ja/desktop' },
            ],
          },
          {
            text: 'リファレンス',
            items: [
              { text: 'アーキテクチャ', link: '/ja/architecture' },
              { text: 'ワイヤープロトコル', link: '/ja/protocol' },
              { text: 'セキュリティ', link: '/ja/security' },
              { text: '設定', link: '/ja/configuration' },
            ],
          },
        ],
        editLink: {
          pattern: 'https://github.com/s-yoshiki/majin/edit/main/docs/:path',
          text: 'GitHubでこのページを編集',
        },
        footer: {
          message: 'MITライセンスで公開しています。',
          copyright: 'majin',
        },
      },
    },
    socialLinks: [{ icon: 'github', link: 'https://github.com/s-yoshiki/majin' }],
    search: {
      provider: 'local',
    },
    outline: {
      level: [2, 3],
    },
  },
});
