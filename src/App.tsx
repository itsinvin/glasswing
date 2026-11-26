import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Gamepad2,
  Github,
  Info,
  Loader2,
  Settings2,
  Sparkles,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Toaster, toast } from "sonner";
import {
  api,
  type AppInfo,
  type Config,
  type DiscoveredConsole,
  type SavedHost,
  type SystemInfo,
} from "@/lib/api";
import { errorText } from "@/lib/format";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Logo } from "@/components/logo";
import { ConsolesPage } from "@/components/consoles-page";
import type { ConsoleRow } from "@/components/console-card";
import { AddConsoleDialog } from "@/components/add-console-dialog";
import { SettingsPage } from "@/components/settings-page";
import { AboutPage } from "@/components/about-page";

type Page = "consoles" | "settings" | "about";

const navItems = [
  { id: "consoles" as const, label: "Consoles", icon: Gamepad2 },
  { id: "settings" as const, label: "Settings", icon: Settings2 },
  { id: "about" as const, label: "About", icon: Info },
];

const pageTitles: Record<Page, string> = {
  consoles: "Consoles",
  settings: "Settings",
  about: "About",
};

export default function App() {
  const [page, setPage] = useState<Page>("consoles");
  const [config, setConfig] = useState<Config | null>(null);
  const [discovered, setDiscovered] = useState<DiscoveredConsole[]>([]);
  const [scanning, setScanning] = useState(false);
  const [lastScan, setLastScan] = useState<number | null>(null);
  const [addOpen, setAddOpen] = useState(false);
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [systemLoading, setSystemLoading] = useState(false);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const autoScanned = useRef(false);

  const refreshSystem = useCallback(async () => {
    setSystemLoading(true);
    try {
      setSystemInfo(await api.systemInfo());
    } catch (error) {
      toast.error("System check failed", {
        description: errorText(error),
      });
    } finally {
      setSystemLoading(false);
    }
  }, []);

  const scan = useCallback(async () => {
    setScanning(true);
    try {
      const direct = config?.hosts.map((host) => host.address) ?? [];
      const found = await api.discover(3000, direct);
      setDiscovered(found);
      setLastScan(Date.now());

      if (found.length === 0) {
        toast.message("No consoles found", {
          description:
            "Make sure the PS5 is on and on the same network, then try again.",
        });
      } else {
        toast.success(
          `Found ${found.length} console${found.length === 1 ? "" : "s"}`,
          { description: found.map((console) => console.name).join(", ") },
        );
      }
    } catch (error) {
      toast.error("Scan failed", { description: errorText(error) });
    } finally {
      setScanning(false);
    }
  }, [config]);

  useEffect(() => {
    let active = true;

    api
      .getConfig()
      .then((value) => {
        if (active) setConfig(value);
      })
      .catch((error) => {
        toast.error("Could not load settings", {
          description: errorText(error),
        });
      });

    api
      .appInfo()
      .then((value) => {
        if (active) setAppInfo(value);
      })
      .catch(() => {
        // The About page has fallback text, so this is not worth a toast.
      });

    void refreshSystem();

    return () => {
      active = false;
    };
  }, [refreshSystem]);

  useEffect(() => {
    if (!config || autoScanned.current) return;
    autoScanned.current = true;
    void scan();
  }, [config, scan]);

  const rows = useMemo<ConsoleRow[]>(() => {
    if (!config) return [];

    const saved: ConsoleRow[] = config.hosts.map((host) => ({
      host,
      live:
        discovered.find(
          (live) =>
            live.address === host.address ||
            (live.host_id !== "" && live.host_id === host.id),
        ) ?? null,
    }));

    const extras: ConsoleRow[] = discovered
      .filter(
        (live) =>
          !config.hosts.some(
            (host) =>
              host.address === live.address ||
              (live.host_id !== "" && host.id === live.host_id),
          ),
      )
      .map((live) => ({ host: null, live }));

    return [...saved, ...extras];
  }, [config, discovered]);

  const saveConsole = useCallback(async (console: DiscoveredConsole) => {
    try {
      const saved = await api.upsertHost({
        name: console.name,
        address: console.address,
        hostId: console.host_id,
        manual: false,
      });
      setConfig(saved);
      toast.success("Console saved", { description: console.name });
    } catch (error) {
      toast.error("Could not save the console", {
        description: errorText(error),
      });
    }
  }, []);

  const wakeConsole = useCallback(async (host: SavedHost) => {
    try {
      const message = await api.wake(host.address, host.regist_key);
      toast.message("Wake packet sent", { description: message });
    } catch (error) {
      toast.error("Could not send the wake packet", {
        description: errorText(error),
      });
    }
  }, []);

  const removeConsole = useCallback(async (host: SavedHost) => {
    if (!window.confirm(`Remove ${host.name} from your list?`)) return;

    try {
      const saved = await api.removeHost(host.id);
      setConfig(saved);
      toast.success("Console removed");
    } catch (error) {
      toast.error("Could not remove the console", {
        description: errorText(error),
      });
    }
  }, []);

  async function openRepository() {
    const url = appInfo?.repository ?? "https://github.com/itsinvin/glasswing";
    try {
      await openUrl(url);
    } catch (error) {
      toast.error("Could not open GitHub", {
        description: errorText(error),
      });
    }
  }

  return (
    <TooltipProvider delayDuration={200}>
      <div className="min-h-screen">
        <div className="mx-auto flex min-h-screen w-full max-w-[1500px]">
          <aside className="sticky top-0 hidden h-screen w-64 shrink-0 flex-col border-r border-white/5 bg-black/20 px-4 py-6 backdrop-blur-xl lg:flex">
            <Logo />

            <nav className="mt-8 space-y-1">
              {navItems.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => setPage(item.id)}
                  className={cn(
                    "flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                    page === item.id
                      ? "bg-white/[0.07] text-foreground"
                      : "text-muted-foreground hover:bg-white/[0.04] hover:text-foreground",
                  )}
                >
                  <item.icon className="h-4 w-4" />
                  {item.label}
                  {page === item.id ? (
                    <span className="ml-auto h-1.5 w-1.5 rounded-full bg-indigo-400" />
                  ) : null}
                </button>
              ))}
            </nav>

            <div className="mt-auto space-y-3">
              <div className="rounded-lg border border-white/5 bg-white/[0.02] p-3">
                <div className="flex items-center gap-2 text-xs font-medium">
                  <Sparkles className="h-3.5 w-3.5 text-indigo-300" />
                  Early build
                </div>
                <p className="mt-1.5 text-[11px] leading-relaxed text-muted-foreground">
                  Console discovery works. Pairing and streaming are next.
                </p>
              </div>

              <button
                type="button"
                onClick={() => void openRepository()}
                className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-[11px] text-muted-foreground transition-colors hover:text-foreground"
              >
                <Github className="h-3.5 w-3.5" />
                Source on GitHub
              </button>

              <p className="px-2 text-[10px] text-muted-foreground/70">
                AGPL-3.0 · v{appInfo?.version ?? "0.1.0"}
              </p>
            </div>
          </aside>

          <main className="min-w-0 flex-1">
            <header className="flex items-center justify-between gap-3 border-b border-white/5 px-5 py-4 sm:px-6 lg:px-10">
              <div className="flex items-center gap-3">
                <div className="lg:hidden">
                  <Logo compact />
                </div>
                <div className="hidden lg:block">
                  <div className="text-sm font-medium">{pageTitles[page]}</div>
                  <div className="text-xs text-muted-foreground">
                    Local network only · Windows 10/11
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-2">
                <Badge
                  variant="outline"
                  className="gap-1.5 border-emerald-400/20 bg-emerald-400/5 text-[10px] text-emerald-300"
                >
                  <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
                  Local only
                </Badge>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => void openRepository()}
                  aria-label="Open GitHub"
                >
                  <Github />
                </Button>
              </div>
            </header>

            <div className="flex gap-1 border-b border-white/5 px-4 py-2 lg:hidden">
              {navItems.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => setPage(item.id)}
                  className={cn(
                    "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs",
                    page === item.id
                      ? "bg-white/[0.07] text-foreground"
                      : "text-muted-foreground",
                  )}
                >
                  <item.icon className="h-3.5 w-3.5" />
                  {item.label}
                </button>
              ))}
            </div>

            <div
              key={page}
              className="animate-in fade-in-0 slide-in-from-bottom-1 px-5 py-6 duration-200 sm:px-6 lg:px-10 lg:py-8"
            >
              {page === "consoles" ? (
                <ConsolesPage
                  config={config}
                  rows={rows}
                  scanning={scanning}
                  lastScan={lastScan}
                  onScan={() => void scan()}
                  onAdd={() => setAddOpen(true)}
                  onSave={saveConsole}
                  onWake={wakeConsole}
                  onRemove={removeConsole}
                />
              ) : page === "settings" ? (
                config ? (
                  <SettingsPage
                    config={config}
                    onConfig={setConfig}
                    systemInfo={systemInfo}
                    systemLoading={systemLoading}
                    onRefreshSystem={() => void refreshSystem()}
                    appInfo={appInfo}
                  />
                ) : (
                  <LoadingPanel />
                )
              ) : (
                <AboutPage appInfo={appInfo} />
              )}
            </div>
          </main>
        </div>

        <AddConsoleDialog
          open={addOpen}
          onOpenChange={setAddOpen}
          onSaved={setConfig}
        />
        <Toaster
          theme="dark"
          position="bottom-right"
          richColors
          closeButton
        />
      </div>
    </TooltipProvider>
  );
}

function LoadingPanel() {
  return (
    <div className="flex h-64 items-center justify-center text-sm text-muted-foreground">
      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
      Loading...
    </div>
  );
}
