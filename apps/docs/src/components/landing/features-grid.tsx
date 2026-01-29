import {
  ArrowRight,
  Cpu,
  Globe,
  Lock,
  Network,
  Shield,
  Zap,
} from 'lucide-react';
import Link from 'next/link';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

const features = [
  {
    icon: Zap,
    title: 'Ultra-Lightweight',
    description:
      '~5MB binary, ~10MB RAM. Run on a 512MB VPS alongside your app.',
    stat: '~10MB',
    statLabel: 'RAM',
  },
  {
    icon: Lock,
    title: 'Auto TLS',
    description:
      "Let's Encrypt certificates issued and renewed automatically. Zero config.",
  },
  {
    icon: Globe,
    title: 'HTTP/2 Ready',
    description:
      'Automatic protocol detection over TLS. HTTP/1.1 fallback for compatibility.',
  },
  {
    icon: Shield,
    title: 'Security Built-in',
    description:
      'HSTS, X-Frame-Options, rate limiting, and path filtering out of the box.',
  },
  {
    icon: Network,
    title: 'WebSocket Support',
    description:
      'Full WebSocket passthrough for HMR, Socket.IO, and real-time apps.',
  },
  {
    icon: Cpu,
    title: 'Zero Runtime',
    description: 'Single static binary. No Node.js, no JVM, no dependencies.',
    badge: 'RUST',
  },
];

export function FeaturesGrid() {
  return (
    <section className="py-20 md:py-24 px-6 md:px-12 bg-secondary/30">
      <div className="max-w-[1400px] mx-auto">
        {/* Section header */}
        <div className="flex flex-col md:flex-row justify-between items-start md:items-end gap-8 mb-16">
          <div>
            <h2 className="text-xs font-black uppercase tracking-[0.4em] text-primary mb-4">
              Core Features
            </h2>
            <h3 className="text-3xl md:text-4xl lg:text-5xl font-extrabold tracking-tighter">
              Everything you need,
              <br />
              <span className="text-muted-foreground">
                nothing you don&apos;t.
              </span>
            </h3>
          </div>
          <p className="text-muted-foreground text-sm max-w-sm">
            Built for developers who want a simple, reliable proxy without the
            bloat.
          </p>
        </div>

        {/* Features grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {features.map((feature) => (
            <Card
              key={feature.title}
              className="bg-card/50 backdrop-blur border-border hover:border-primary/50 transition-all group py-8"
            >
              <CardHeader className="pb-0">
                <div className="flex items-center justify-between mb-4">
                  <div className="size-12 bg-secondary border border-border rounded-xl flex items-center justify-center group-hover:border-primary/30 transition-colors">
                    <feature.icon className="size-6 text-muted-foreground group-hover:text-primary transition-colors" />
                  </div>
                  {feature.badge && (
                    <div className="px-2 py-0.5 bg-secondary border border-border text-muted-foreground text-[10px] font-mono rounded">
                      {feature.badge}
                    </div>
                  )}
                  {feature.stat && (
                    <div className="text-right">
                      <p className="text-xl font-black">{feature.stat}</p>
                      <p className="text-[10px] font-bold uppercase tracking-widest text-primary">
                        {feature.statLabel}
                      </p>
                    </div>
                  )}
                </div>
                <CardTitle className="text-lg">{feature.title}</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  {feature.description}
                </p>
              </CardContent>
            </Card>
          ))}

          {/* CTA Card */}
          <Card className="bg-primary border-primary hover:brightness-110 transition-all group py-8 cursor-pointer">
            <Link
              href="/getting-started/installation"
              className="h-full flex flex-col"
            >
              <CardHeader className="pb-0 flex-1">
                <div className="size-10 bg-black/10 rounded-lg flex items-center justify-center mb-4">
                  <ArrowRight className="size-5 text-primary-foreground" />
                </div>
                <CardTitle className="text-xl font-black text-primary-foreground">
                  Deploy in Minutes
                </CardTitle>
                <p className="text-sm text-primary-foreground/70 font-medium mt-2">
                  One Docker command. Auto TLS. Production ready.
                </p>
              </CardHeader>
              <CardContent className="pt-4">
                <span className="text-[10px] font-bold uppercase tracking-widest text-primary-foreground/60">
                  Zero Configuration Required
                </span>
              </CardContent>
            </Link>
          </Card>
        </div>
      </div>
    </section>
  );
}
