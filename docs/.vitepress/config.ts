import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'

export default withMermaid(
  defineConfig({
    title: 'matrix-gen',
    description: 'Multi-agent social simulator for synthesizing diverse instruction data',
    base: '/matrix-gen/',

    themeConfig: {
      nav: [
        { text: 'guide', link: '/introduction' },
        { text: 'rhi', link: 'https://rhi.zone/' },
      ],
      sidebar: [
        {
          text: 'guide',
          items: [
            { text: 'introduction', link: '/introduction' },
          ],
        },
      ],
    },
  })
)
