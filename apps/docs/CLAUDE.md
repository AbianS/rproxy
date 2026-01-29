# rproxy Documentation - Technical Context

> **Context Note**: This is the **docs-specific context** for rproxy.
> - Root context: `/CLAUDE.md`
> - Server context: `apps/server/CLAUDE.md`

## Overview

Documentation site for rproxy built with Next.js 16 and Nextra. Features a custom landing page and comprehensive technical documentation.

## Tech Stack

- **Framework**: Next.js 16.1 (App Router)
- **Docs Theme**: Nextra 4.6
- **Styling**: Tailwind CSS 4.1
- **UI Components**: Radix UI + shadcn/ui patterns
- **Icons**: Lucide React

## Directory Structure

```
apps/docs/
├── content/                 # MDX documentation files
│   ├── index.mdx           # Landing page
│   ├── getting-started/
│   │   ├── overview.mdx
│   │   └── installation.mdx
│   ├── configuration/
│   │   └── environment.mdx
│   └── reference/
│       └── architecture.mdx
├── src/
│   ├── app/
│   │   ├── layout.tsx      # Root layout with Nextra
│   │   ├── globals.css     # Tailwind + theme colors
│   │   └── [[...mdxPath]]/ # Dynamic MDX routing
│   ├── components/
│   │   ├── landing/        # Landing page components
│   │   │   ├── landing-page.tsx
│   │   │   ├── hero-section.tsx
│   │   │   ├── navbar.tsx
│   │   │   ├── terminal-preview.tsx
│   │   │   └── features-grid.tsx
│   │   └── ui/             # shadcn/ui components
│   │       ├── badge.tsx
│   │       ├── button.tsx
│   │       └── card.tsx
│   └── lib/
│       └── utils.ts        # cn() utility
├── mdx-components.tsx      # MDX component overrides
├── next.config.mjs         # Nextra configuration
└── package.json
```

## Color Theme

The docs use a **cyan/teal** color scheme to represent network/proxy concepts:

```css
/* Light mode */
--primary: oklch(0.65 0.15 195); /* Cyan */

/* Dark mode */
--primary: oklch(0.78 0.15 195); /* Bright cyan */
--background: oklch(0.14 0.01 240); /* Deep blue-black */
```

## Development

```bash
# Install dependencies
pnpm install

# Start dev server (port 3001)
pnpm dev

# Build for production
pnpm build
```

## Adding Documentation

1. Create MDX file in `content/` directory
2. Add frontmatter with title and description:
   ```mdx
   ---
   title: Page Title
   description: Page description for SEO
   ---
   ```
3. Nextra automatically adds to sidebar based on file structure

## Code Blocks

Use `filename` attribute for file names (not `title`):

```mdx
```yaml filename="docker-compose.yml"
services:
  proxy:
    image: rproxy:latest
```
```

## Landing Page Components

The landing page uses custom React components imported in `content/index.mdx`:

- `LandingNavbar` - Sticky header with mobile menu
- `HeroSection` - Main headline with CTAs
- `TerminalPreview` - Docker compose example
- `FeaturesGrid` - Feature cards grid

## Static Export

Configured for static export to GitHub Pages:

```js
// next.config.mjs
export default withNextra({
  output: 'export',
  basePath: process.env.GITHUB_ACTIONS ? '/rproxy' : '',
});
```

## Skills

When working on docs, leverage:

- **vercel-react-best-practices** - Next.js patterns
- **typescript-strict** - Type-safe TypeScript
