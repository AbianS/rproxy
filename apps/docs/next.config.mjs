import nextra from 'nextra';

const withNextra = nextra({
  contentDirBasePath: '/',
  defaultShowCopyCode: true,
});

const basePath = process.env.GITHUB_ACTIONS ? '/rproxy' : '';

export default withNextra({
  output: 'export',
  images: { unoptimized: true },
  basePath,
  assetPrefix: process.env.GITHUB_ACTIONS ? '/rproxy/' : '',
  env: {
    NEXT_PUBLIC_BASE_PATH: basePath,
  },
});
