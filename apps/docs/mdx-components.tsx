import { useMDXComponents as getDocsMDXComponents } from 'nextra-theme-docs';

export const useMDXComponents = (components: any = {}) =>
  getDocsMDXComponents({ ...components });
