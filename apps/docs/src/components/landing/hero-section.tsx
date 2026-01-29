import { ArrowRight, Github } from 'lucide-react';
import Link from 'next/link';

import { Button } from '@/components/ui/button';

export function HeroSection() {
  return (
    <section className="relative overflow-hidden py-20 md:py-32">
      {/* Gradient background effects */}
      <div className="absolute inset-0 -z-10">
        <div className="absolute top-1/4 left-1/2 -translate-x-1/2 w-[600px] h-[400px] bg-primary/10 rounded-full blur-[120px]" />
        <div className="absolute top-1/3 left-1/3 w-[400px] h-[300px] bg-primary/5 rounded-full blur-[100px]" />
      </div>

      <div className="max-w-[1400px] mx-auto px-6 md:px-12 text-center">
        {/* Badge */}
        <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-primary/10 border border-primary/20 mb-8">
          <span className="size-1.5 rounded-full bg-primary animate-pulse" />
          <span className="text-[10px] font-mono font-bold tracking-[0.2em] text-primary uppercase">
            Ultra-Lightweight Reverse Proxy
          </span>
        </div>

        {/* Heading */}
        <h1 className="text-5xl md:text-7xl lg:text-8xl font-black tracking-tighter leading-[0.9] mb-8">
          Minimal proxy for
          <br />
          <span className="text-primary italic">self-hosted apps</span>
        </h1>

        {/* Subtitle */}
        <p className="text-muted-foreground text-lg md:text-xl max-w-2xl mx-auto mb-10 leading-relaxed">
          High-performance Rust reverse proxy (~5MB binary, ~10MB RAM). TLS with
          Let&apos;s Encrypt, rate limiting, WebSocket support. Deploy with
          Docker in minutes.
        </p>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
          <Button
            asChild
            size="lg"
            className="w-full sm:w-auto px-8 py-6 text-sm font-black uppercase tracking-widest shadow-[0_0_40px_-10px_rgba(56,189,248,0.3)]"
          >
            <Link href="/getting-started/overview">
              Get Started <ArrowRight className="ml-2 size-4" />
            </Link>
          </Button>
          <Button
            variant="outline"
            asChild
            size="lg"
            className="w-full sm:w-auto px-8 py-6 text-sm font-bold uppercase tracking-widest"
          >
            <Link href="https://github.com/AbianS/rproxy" target="_blank">
              <Github className="mr-2 size-4" /> View on GitHub
            </Link>
          </Button>
        </div>

        {/* Stats */}
        <div className="flex flex-wrap justify-center gap-8 mt-16 pt-8 border-t border-border/50">
          <div className="text-center">
            <p className="text-3xl font-black text-primary">~5MB</p>
            <p className="text-xs font-bold uppercase tracking-widest text-muted-foreground">
              Binary Size
            </p>
          </div>
          <div className="text-center">
            <p className="text-3xl font-black text-primary">~10MB</p>
            <p className="text-xs font-bold uppercase tracking-widest text-muted-foreground">
              RAM Usage
            </p>
          </div>
          <div className="text-center">
            <p className="text-3xl font-black text-primary">HTTP/2</p>
            <p className="text-xs font-bold uppercase tracking-widest text-muted-foreground">
              Auto-detect
            </p>
          </div>
          <div className="text-center">
            <p className="text-3xl font-black text-primary">0 deps</p>
            <p className="text-xs font-bold uppercase tracking-widest text-muted-foreground">
              Runtime
            </p>
          </div>
        </div>
      </div>

      {/* Bottom gradient line */}
      <div className="absolute bottom-0 left-0 w-full h-px bg-gradient-to-r from-transparent via-border to-transparent" />
    </section>
  );
}
