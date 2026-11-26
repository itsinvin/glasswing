import {
  Check,
  ExternalLink,
  Github,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import type { AppInfo } from "@/lib/api";
import { errorText } from "@/lib/format";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Logo } from "@/components/logo";

interface AboutPageProps {
  appInfo: AppInfo | null;
}

const stack = [
  "Rust",
  "Tauri 2",
  "React",
  "shadcn/ui",
  "wgpu",
  "FFmpeg",
  "SDL2",
  "AGPL-3.0",
];

const links = [
  {
    title: "chiaki-ng",
    detail: "Open-source protocol reference for PlayStation Remote Play.",
    url: "https://github.com/chiaki-ng/chiaki-ng",
  },
  {
    title: "KyberNow",
    detail: "Rust notes for discovery, wake, and the stream protocol.",
    url: "https://github.com/MonsieurNours/kybernow",
  },
  {
    title: "shadcn/ui",
    detail: "The UI building blocks used by this app.",
    url: "https://github.com/shadcn-ui/ui",
  },
  {
    title: "Tauri",
    detail: "Small desktop shell that connects the web UI to Rust.",
    url: "https://tauri.app",
  },
];

export function AboutPage({ appInfo }: AboutPageProps) {
  const repository =
    appInfo?.repository ?? "https://github.com/itsinvin/glasswing";

  async function open(url: string) {
    try {
      await openUrl(url);
    } catch (error) {
      toast.error("Could not open the link", {
        description: errorText(error),
      });
    }
  }

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      <div className="flex flex-col items-start gap-5 sm:flex-row sm:items-center">
        <Logo size="lg" compact />
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="text-2xl font-semibold tracking-tight">Glasswing</h1>
            <Badge
              variant="outline"
              className="border-white/10 font-mono text-[10px]"
            >
              v{appInfo?.version ?? "0.1.0"}
            </Badge>
            <Badge
              variant="outline"
              className="border-indigo-400/20 bg-indigo-400/5 text-[10px] text-indigo-300"
            >
              Early build
            </Badge>
          </div>
          <p className="mt-2 max-w-xl text-sm text-muted-foreground">
            A low-latency PS5 Remote Play client for Windows. Built in Rust, with
            a web UI you can read and change.
          </p>
        </div>
      </div>

      <Card className="border-white/10 bg-white/[0.03]">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Sparkles className="h-4 w-4 text-indigo-300" />
            What works today
          </CardTitle>
          <CardDescription>
            This is the first slice. It is useful already, but it is not a full
            stream client yet.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3 text-sm">
          <CheckRow text="Find a PS5 on your local network and read its state." />
          <CheckRow text="Save consoles, stream defaults, and settings on this PC." />
          <CheckRow text="Check for an NVIDIA GPU, FFmpeg, and NVDEC." />
          <div className="rounded-lg border border-white/5 bg-black/20 p-3 text-xs text-muted-foreground">
            <span className="font-medium text-foreground">Next: </span>
            pair with a PIN, then request a session and start the first 1080p60
            stream.
          </div>
        </CardContent>
      </Card>

      <Card className="border-white/10 bg-white/[0.03]">
        <CardHeader>
          <CardTitle className="text-base">Built with</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-1.5">
          {stack.map((item) => (
            <Badge
              key={item}
              variant="outline"
              className="border-white/10 bg-white/[0.02] text-[11px] text-muted-foreground"
            >
              {item}
            </Badge>
          ))}
        </CardContent>
      </Card>

      <Card className="border-white/10 bg-white/[0.03]">
        <CardHeader>
          <CardTitle className="text-base">Credits</CardTitle>
          <CardDescription>
            Glasswing stands on open-source protocol work.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-2">
          {links.map((link) => (
            <button
              key={link.title}
              type="button"
              onClick={() => void open(link.url)}
              className="flex w-full items-center justify-between gap-3 rounded-lg border border-white/5 bg-white/[0.02] px-3.5 py-3 text-left transition-colors hover:bg-white/[0.05]"
            >
              <div>
                <div className="text-sm font-medium">{link.title}</div>
                <div className="mt-0.5 text-xs text-muted-foreground">
                  {link.detail}
                </div>
              </div>
              <ExternalLink className="h-4 w-4 shrink-0 text-muted-foreground" />
            </button>
          ))}
        </CardContent>
      </Card>

      <Card className="border-white/10 bg-white/[0.03]">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <ShieldCheck className="h-4 w-4 text-indigo-300" />
            License and disclaimer
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm text-muted-foreground">
          <p>
            Glasswing is free software under the GNU Affero General Public
            License v3.0. If you run a modified version as a network service,
            you must share the source.
          </p>
          <p>
            This project is not affiliated with or endorsed by Sony. Use it with
            consoles and games you own.
          </p>
          <Button variant="outline" onClick={() => void open(repository)}>
            <Github />
            View the source
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}

function CheckRow({ text }: { text: string }) {
  return (
    <div className="flex items-start gap-2.5">
      <span className="mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-emerald-400/10 text-emerald-300 ring-1 ring-inset ring-emerald-400/20">
        <Check className="h-3 w-3" />
      </span>
      <span className="text-muted-foreground">{text}</span>
    </div>
  );
}
