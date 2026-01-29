import { Shield } from 'lucide-react';
import { Geist, Geist_Mono } from 'next/font/google';
import { Head } from 'nextra/components';
import { getPageMap } from 'nextra/page-map';
import { Footer, Layout, Navbar } from 'nextra-theme-docs';
import './globals.css';

const geistSans = Geist({
  variable: '--font-geist-sans',
  subsets: ['latin'],
});

const geistMono = Geist_Mono({
  variable: '--font-geist-mono',
  subsets: ['latin'],
});

export const metadata = {
  title: {
    default: 'rproxy Documentation',
    template: '%s - rproxy',
  },
  description:
    'Minimal high-performance reverse proxy for self-hosted applications',
};

const logo = (
  <span className="flex items-center gap-2 font-bold">
    <span className="size-6 bg-primary rounded-sm flex items-center justify-center">
      <Shield className="size-4 text-primary-foreground" />
    </span>
    <span className="text-sm font-extrabold tracking-tight uppercase">
      rproxy
    </span>
  </span>
);

export default async function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const pageMap = await getPageMap();

  return (
    <html lang="en" suppressHydrationWarning>
      <Head faviconGlyph="R" />
      <body className={`${geistSans.variable} ${geistMono.variable} font-sans`}>
        <Layout
          pageMap={pageMap}
          docsRepositoryBase="https://github.com/AbianS/rproxy/tree/main/apps/docs"
          navbar={
            <Navbar
              logo={logo}
              projectLink="https://github.com/AbianS/rproxy"
            />
          }
          footer={<Footer>GPL-3.0 {new Date().getFullYear()} rproxy</Footer>}
        >
          {children}
        </Layout>
      </body>
    </html>
  );
}
