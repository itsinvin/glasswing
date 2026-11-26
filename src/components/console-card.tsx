import { Gamepad2, Plus, Power, Trash2 } from "lucide-react";
import type { DiscoveredConsole, SavedHost } from "@/lib/api";
import { stateLabel, timeAgo } from "@/lib/format";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export interface ConsoleRow {
  host: SavedHost | null;
  live: DiscoveredConsole | null;
}

interface ConsoleCardProps {
  row: ConsoleRow;
  onSave: (console: DiscoveredConsole) => void;
  onWake: (host: SavedHost) => void;
  onRemove: (host: SavedHost) => void;
}

export function ConsoleCard({
  row,
  onSave,
  onWake,
  onRemove,
}: ConsoleCardProps) {
  const { host, live } = row;
  const name = live?.name || host?.name || "PlayStation 5";
  const state = live?.state ?? "offline";
  const address = live?.address || host?.address || "Unknown";
  const isReady = state === "ready";
  const isStandby = state === "standby";
  const lastSeen = host ? timeAgo(host.last_seen) : null;

  const dotClass = isReady
    ? "bg-emerald-400 shadow-[0_0_10px_rgba(52,211,153,0.75)]"
    : isStandby
      ? "bg-amber-400"
      : "bg-zinc-500";

  return (
    <Card className="group relative overflow-hidden border-white/10 bg-white/[0.03] transition-colors hover:border-white/20 hover:bg-white/[0.045]">
      <div className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-indigo-400/60 to-transparent opacity-0 transition-opacity group-hover:opacity-100" />

      <CardHeader className="flex flex-row items-start justify-between space-y-0 pb-3">
        <div className="flex min-w-0 items-center gap-3">
          <div
            className={cn(
              "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg ring-1 ring-inset",
              isReady
                ? "bg-emerald-400/10 ring-emerald-400/20"
                : isStandby
                  ? "bg-amber-400/10 ring-amber-400/20"
                  : "bg-white/[0.04] ring-white/10",
            )}
          >
            <Gamepad2
              className={cn(
                "h-5 w-5",
                isReady
                  ? "text-emerald-300"
                  : isStandby
                    ? "text-amber-300"
                    : "text-zinc-400",
              )}
            />
          </div>

          <div className="min-w-0">
            <CardTitle className="truncate text-base">{name}</CardTitle>
            <div className="mt-1 flex items-center gap-1.5 text-xs text-muted-foreground">
              <span className={cn("h-1.5 w-1.5 rounded-full", dotClass)} />
              {stateLabel(state)}
              {live ? (
                <span className="text-muted-foreground/60">· found now</span>
              ) : lastSeen ? (
                <span className="text-muted-foreground/60">· {lastSeen}</span>
              ) : null}
            </div>
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-1.5">
          {live?.running_app_name ? (
            <Badge
              variant="outline"
              className="border-indigo-400/20 bg-indigo-400/5 text-[10px] text-indigo-300"
            >
              In game
            </Badge>
          ) : null}
          <Badge
            variant="outline"
            className="border-white/10 text-[10px] uppercase tracking-wider text-muted-foreground"
          >
            PS5
          </Badge>
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-3 text-xs">
          <div>
            <div className="text-muted-foreground">Address</div>
            <div className="mt-1 truncate font-mono text-foreground/90">
              {address}
            </div>
          </div>
          <div>
            <div className="text-muted-foreground">System</div>
            <div className="mt-1 font-mono text-foreground/90">
              {live?.system_version ?? "—"}
            </div>
          </div>
        </div>

        {live?.running_app_name ? (
          <div className="rounded-md border border-white/5 bg-white/[0.03] px-3 py-2 text-xs">
            <span className="text-muted-foreground">Playing </span>
            <span className="font-medium text-foreground">
              {live.running_app_name}
            </span>
          </div>
        ) : null}

        {!host && live ? (
          <p className="text-xs text-muted-foreground">
            Found on your network. Add it to keep it in your list.
          </p>
        ) : null}

        {host && !live ? (
          <p className="text-xs text-muted-foreground">
            Not seen in the last scan. Turn the console on or check the network.
          </p>
        ) : null}

        {host && isStandby && !host.regist_key ? (
          <p className="text-xs text-amber-300/80">
            Wake needs pairing. That is the next step.
          </p>
        ) : null}

        <div className="flex items-center gap-2">
          {!host ? (
            <Button size="sm" onClick={() => live && onSave(live)}>
              <Plus />
              Add console
            </Button>
          ) : (
            <>
              <Button size="sm" variant="outline" onClick={() => onWake(host)}>
                <Power />
                Wake
              </Button>
              <Button
                size="sm"
                variant="ghost"
                className="text-muted-foreground hover:text-destructive"
                onClick={() => onRemove(host)}
                aria-label={`Remove ${name}`}
              >
                <Trash2 />
              </Button>
            </>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
