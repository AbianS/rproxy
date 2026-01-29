export default {
  index: {
    title: 'Home',
    type: 'page',
    display: 'hidden',
    theme: {
      layout: 'full',
      sidebar: false,
      toc: false,
      breadcrumb: false,
      pagination: false,
      navbar: false,
      footer: false,
    },
  },
  documentation: {
    title: 'Documentation',
    type: 'page',
    href: '/getting-started/overview',
  },
  'getting-started': 'Getting Started',
  configuration: 'Configuration',
  '---': {
    type: 'separator',
  },
  reference: {
    title: 'Reference',
    type: 'menu',
    items: {
      architecture: { title: 'Architecture', href: '/reference/architecture' },
    },
  },
};
