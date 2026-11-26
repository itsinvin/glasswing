import { useEffect, useState } from "react";
import {
  Copy,
  Cpu,
  ExternalLink,
  Gauge,
  HardDrive,
  Loader2,
  RefreshCw,
  Save,
  ShieldCheck,
  Zap,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import { api, type AppInfo, type Config, type SystemInfo } from "@/lib/api";
import { errorText } from "@/lib/format";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";

interface SettingsPageProps {
  config: Config;
  onConfig: (config: Config) => void;
  systemInfo: SystemInfo | null;
  systemLoading: boolean;
  onRefreshSystem: () => void;
  appInfo: AppInfo | null;
}

export function SettingsPage({
  config,
  onConfig,
  systemInfo,
  systemLoading,
  onRefreshSystem,
  appInfo,
}: SettingsPageProps) {
  const [draft, setDraft] = useState<Config>(config);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setDraft(config);
  }, [config]);

  const dirty = JSON.stringify(draft) !== JSON.stringify(config);

  function updateStream(patch: Partial<Config["stream"]>) {
    setDraft((current) => ({
      ...current,
      stream: { ...current.stream, ...patch },
    }));
  }

  function updatePerformance(patch: Partial<Config["performance"]>) {
    setDraft((current) => ({
      ...current,
      performance: { ...current.performance, ...patch },
    }));
  }

  async function save() {
    setSaving(true);
    try {
      const saved = await api.saveConfig(draft);
      onConfig(saved);
      toast.success("Settings saved");
    } catch (error) {
      toast.error("Could not save settings", {
        description: errorText(error),
      });
    } finally {
      setSaving(false);
    }
  }

  async function copy(text: string, label: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(`${label} copied`);
    } catch {
      toast.error("Could not copy to the clipboard");
    }
  }

  async function openPerformanceDocs() {
    try {
      await openUrl(
        "https://github.com/itsinvin/glasswing/blob/main/docs/PERFORMANCE.md",
      );
    } catch (error) {
      toast.error("Could not open the link", {
        description: errorText(error),
      });
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Choose how the stream should look and how the PC should work.
          </p>
        </div>

        <div className="flex items-center gap-2">
          {dirty ? (
            <Badge
              variant="outline"
              className="border-amber-400/20 bg-amber-400/5 text-amber-300"
            >
              Unsaved changes
            </Badge>
          ) : null}
          <Button onClick={save} disabled={!dirty || saving}>
            {saving ? <Loader2 className="animate-spin" /> : <Save />}
            Save
          </Button>
        </div>
      </div>

      <div className="grid gap-5 xl:grid-cols-[1.1fr_0.9fr]">
        <div className="space-y-5">
          <Card className="border-white/10 bg-white/[0.03]">
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <ShieldCheck className="h-4 w-4 text-indigo-300" />
                PlayStation account
              </CardTitle>
              <CardDescription>
                Used to pair with your own PS5. It stays on this PC.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="space-y-1.5">
                <Label htmlFor="psn-account">PSN account ID</Label>
                <Input
                  id="psn-account"
                  value={draft.psn_account_id ?? ""}
                  onChange={(event) =>
                    setDraft({
                      ...draft,
                      psn_account_id: event.target.value || null,
                    })
                  }
                  placeholder="Paste the base64 ID"
                  spellCheck={false}
                  autoComplete="off"
                  className="font-mono text-xs"
                />
                <p className="text-xs text-muted-foreground">
                  Pairing is the next milestone. You can paste it now; the app
                  will use it then.
                </p>
              </div>
            </CardContent>
          </Card>

          <Card className="border-white/10 bg-white/[0.03]">
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <Gauge className="h-4 w-4 text-indigo-300" />
                Stream defaults
              </CardTitle>
              <CardDescription>
                These are the settings for the first playable stream.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-5">
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-1.5">
                  <Label htmlFor="resolution">Resolution</Label>
                  <select
                    id="resolution"
                    value={draft.stream.resolution}
                    onChange={(event) =>
                      updateStream({ resolution: event.target.value })
                    }
                    className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  >
                    <option value="1080p">1080p</option>
                    <option value="720p">720p</option>
                  </select>
                </div>

                <div className="space-y-1.5">
                  <Label>Frame rate</Label>
                  <div className="flex rounded-md border border-input bg-background/40 p-0.5">
                    {[30, 60].map((fps) => (
                      <button
                        key={fps}
                        type="button"
                        onClick={() => updateStream({ fps })}
                        className={cn(
                          "h-8 flex-1 rounded text-xs font-medium transition-colors",
                          draft.stream.fps === fps
                            ? "bg-secondary text-foreground shadow-sm"
                            : "text-muted-foreground hover:text-foreground",
                        )}
                      >
                        {fps} fps
                      </button>
                    ))}
                  </div>
                </div>
              </div>

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label>Bitrate</Label>
                  <span className="font-mono text-xs text-muted-foreground">
                    {draft.stream.bitrate_mbps} Mbps
                  </span>
                </div>
                <Slider
                  value={[draft.stream.bitrate_mbps]}
                  min={5}
                  max={50}
                  step={1}
                  onValueChange={([value]) =>
                    updateStream({ bitrate_mbps: value })
                  }
                />
                <p className="text-xs text-muted-foreground">
                  Higher is sharper and uses more network. 15 Mbps is a good
                  start for 1080p60.
                </p>
              </div>

              <Separator className="bg-white/5" />

              <div className="flex items-center justify-between gap-4 text-sm">
                <div>
                  <div className="font-medium">Codec</div>
                  <div className="text-xs text-muted-foreground">
                    H.264 works on every PS5 and is the first target.
                  </div>
                </div>
                <Badge
                  variant="outline"
                  className="border-white/10 font-mono text-[10px]"
                >
                  H.264
                </Badge>
              </div>

              <div className="flex items-center justify-between gap-4 text-sm">
                <div>
                  <div className="font-medium">HDR</div>
                  <div className="text-xs text-muted-foreground">
                    HDR comes after the first SDR stream works.
                  </div>
                </div>
                <Badge
                  variant="outline"
                  className="border-white/10 text-[10px] text-muted-foreground"
                >
                  Later
                </Badge>
              </div>
            </CardContent>
          </Card>
        </div>

        <div className="space-y-5">
          <Card className="border-white/10 bg-white/[0.03]">
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <Zap className="h-4 w-4 text-cyan-300" />
                Performance
              </CardTitle>
              <CardDescription>
                NVIDIA decode and a short present path keep delay low.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <ToggleRow
                title="Prefer NVIDIA decode"
                description="Use NVDEC or D3D11VA when the GPU supports it."
                checked={draft.performance.prefer_nvidia}
                onCheckedChange={(value) =>
                  updatePerformance({ prefer_nvidia: value })
                }
              />
              <ToggleRow
                title="Low-latency present"
                description="Skip extra frame queues where the driver allows it."
                checked={draft.performance.low_latency_present}
                onCheckedChange={(value) =>
                  updatePerformance({ low_latency_present: value })
                }
              />

              <Separator className="bg-white/5" />

              <div className="rounded-lg border border-white/5 bg-black/20 p-3.5">
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2 text-xs font-medium">
                    <Cpu className="h-3.5 w-3.5 text-muted-foreground" />
                    System check
                  </div>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-7 px-2 text-xs"
                    onClick={onRefreshSystem}
                    disabled={systemLoading}
                  >
                    {systemLoading ? (
                      <Loader2 className="animate-spin" />
                    ) : (
                      <RefreshCw />
                    )}
                    Refresh
                  </Button>
                </div>

                {systemLoading && !systemInfo ? (
                  <div className="mt-3 space-y-2">
                    <div className="h-3 w-2/3 animate-pulse rounded bg-white/10" />
                    <div className="h-3 w-1/2 animate-pulse rounded bg-white/10" />
                  </div>
                ) : systemInfo ? (
                  <div className="mt-3 space-y-3 text-xs">
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <div className="text-muted-foreground">GPU</div>
                        <div className="mt-0.5 truncate font-medium text-foreground">
                          {systemInfo.gpu?.name ?? "No NVIDIA GPU found"}
                        </div>
                        {systemInfo.gpu ? (
                          <div className="mt-0.5 text-muted-foreground">
                            Driver {systemInfo.gpu.driver} ·{" "}
                            {systemInfo.gpu.memory}
                          </div>
                        ) : null}
                      </div>
                    </div>

                    <div className="flex flex-wrap gap-1.5">
                      <Badge
                        variant="outline"
                        className={cn(
                          "text-[10px]",
                          systemInfo.nvdec
                            ? "border-emerald-400/20 bg-emerald-400/5 text-emerald-300"
                            : "border-amber-400/20 bg-amber-400/5 text-amber-300",
                        )}
                      >
                        {systemInfo.nvdec ? "NVDEC ready" : "NVDEC missing"}
                      </Badge>
                      <Badge
                        variant="outline"
                        className={cn(
                          "text-[10px]",
                          systemInfo.ffmpeg
                            ? "border-emerald-400/20 bg-emerald-400/5 text-emerald-300"
                            : "border-amber-400/20 bg-amber-400/5 text-amber-300",
                        )}
                      >
                        {systemInfo.ffmpeg ? "FFmpeg found" : "FFmpeg missing"}
                      </Badge>
                    </div>

                    <p className="text-muted-foreground">
                      Video decoding is not built yet. This check makes sure the
                      PC is ready for it.
                    </p>
                  </div>
                ) : (
                  <p className="mt-3 text-xs text-muted-foreground">
                    Could not run the check.
                  </p>
                )}
              </div>

              <Button
                variant="ghost"
                size="sm"
                className="w-full justify-between text-xs text-muted-foreground hover:text-foreground"
                onClick={() => void openPerformanceDocs()}
              >
                Read the NVIDIA performance notes
                <ExternalLink />
              </Button>
            </CardContent>
          </Card>

          <Card className="border-white/10 bg-white/[0.03]">
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <HardDrive className="h-4 w-4 text-indigo-300" />
                Data
              </CardTitle>
              <CardDescription>
                Everything stays on this PC. No cloud account is used.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4 text-xs">
              <PathRow
                label="Settings file"
                value={appInfo?.config_path ?? "Loading..."}
                onCopy={() =>
                  appInfo && copy(appInfo.config_path, "Settings path")
                }
              />
              <PathRow
                label="Logs"
                value={appInfo?.log_path ?? "Loading..."}
                onCopy={() => appInfo && copy(appInfo.log_path, "Logs path")}
              />

              <Separator className="bg-white/5" />

              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Version</span>
                <span className="font-mono">
                  {appInfo?.version ?? "0.1.0"}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">License</span>
                <span className="font-mono">
                  {appInfo?.license ?? "AGPL-3.0"}
                </span>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}

function ToggleRow({
  title,
  description,
  checked,
  onCheckedChange,
}: {
  title: string;
  description: string;
  checked: boolean;
  onCheckedChange: (value: boolean) => void;
}) {
  return (
    <div className="flex items-start justify-between gap-4 rounded-lg border border-white/5 bg-white/[0.02] p-3.5">
      <div>
        <div className="text-sm font-medium">{title}</div>
        <div className="mt-0.5 text-xs text-muted-foreground">
          {description}
        </div>
      </div>
      <Switch checked={checked} onCheckedChange={onCheckedChange} />
    </div>
  );
}

function PathRow({
  label,
  value,
  onCopy,
}: {
  label: string;
  value: string;
  onCopy: () => void;
}) {
  return (
    <div>
      <div className="flex items-center justify-between gap-2">
        <span className="text-muted-foreground">{label}</span>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-[11px]"
          onClick={onCopy}
        >
          <Copy />
          Copy
        </Button>
      </div>
      <div className="mt-1 break-all rounded-md border border-white/5 bg-black/20 px-2.5 py-2 font-mono text-[11px] text-foreground/80">
        {value}
      </div>
    </div>
  );
}
