import { Clock3, Loader2, Plus, Radar, Sparkles } from "lucide-react";
import type { Config, DiscoveredConsole, SavedHost } from "@/lib/api";
import { timeAgo } from "@/lib/format";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { ConsoleCard, type ConsoleRow } from "@/components/console-card";

interface ConsolesPageProps {
  config: Config | null;
  rows: ConsoleRow[];
  scanning: boolean;
  lastScan: number | null;
  onScan: () => void;
  onAdd: () => void;
  onSave: (console: DiscoveredConsole) => void;
  onWake: (host: SavedHost) => void;
  onRemove: (host: SavedHost) => void;
}

export function ConsolesPage({
  config,
  rows,
  scanning,
  lastScan,
  onScan,
  onAdd,
  onSave,
  onWake,
  onRemove,
}: ConsolesPageProps) {
  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            Your consoles
          </h1>
          <p className="mt-1 max-w-xl text-sm text-muted-foreground">
            Find your PS5 on this network. Save it, wake it, and soon start a
            game stream.
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button variant="outline" onClick={onAdd}>
            <Plus />
            Add by IP
          </Button>
          <Button onClick={onScan} disabled={scanning}>
            {scanning ? <Loader2 className="animate-spin" /> : <Radar />}
            {scanning ? "Scanning" : "Scan network"}
          </Button>
        </div>
      </div>

      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <Clock3 className="h-3.5 w-3.5" />
        <span>
          {lastScan
            ? `Last scan ${timeAgo(Math.floor(lastScan / 1000))}`
            : "Not scanned yet"}
        </span>
        <span className="text-muted-foreground/40">·</span>
        <span>UDP 9302</span>
      </div>

      <div className="flex items-start gap-3 rounded-xl border border-indigo-400/20 bg-indigo-400/[0.06] p-4">
        <Sparkles className="mt-0.5 h-4 w-4 shrink-0 text-indigo-300" />
        <div className="text-sm">
          <p className="font-medium">Step 1 is working.</p>
          <p className="mt-0.5 text-muted-foreground">
            Discovery is live. Pairing with a PIN and game streaming are the
            next milestones.
          </p>
        </div>
      </div>

      {!config ? (
        <LoadingCards />
      ) : rows.length === 0 ? (
        <EmptyState scanning={scanning} onScan={onScan} onAdd={onAdd} />
      ) : (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {rows.map((row) => (
            <ConsoleCard
              key={row.host?.id ?? `live:${row.live?.address ?? "unknown"}`}
              row={row}
              onSave={onSave}
              onWake={onWake}
              onRemove={onRemove}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function LoadingCards() {
  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
      {[0, 1, 2].map((index) => (
        <div
          key={index}
          className="h-56 animate-pulse rounded-xl border border-white/5 bg-white/[0.03]"
        />
      ))}
    </div>
  );
}

function EmptyState({
  scanning,
  onScan,
  onAdd,
}: {
  scanning: boolean;
  onScan: () => void;
  onAdd: () => void;
}) {
  return (
    <div className="flex flex-col items-center justify-center rounded-xl border border-dashed border-white/10 bg-white/[0.02] px-6 py-16 text-center">
      <div className="flex h-14 w-14 items-center justify-center rounded-full bg-indigo-400/10 ring-1 ring-inset ring-indigo-400/20">
        <Radar
          className={cn("h-6 w-6 text-indigo-300", scanning && "animate-spin")}
        />
      </div>
      <h2 className="mt-4 text-base font-medium">No consoles yet</h2>
      <p className="mt-1 max-w-sm text-sm text-muted-foreground">
        Turn on your PS5, make sure Remote Play is allowed, and scan again. You
        can also add an address by hand.
      </p>
      <div className="mt-5 flex flex-wrap justify-center gap-2">
        <Button onClick={onScan} disabled={scanning}>
          {scanning ? <Loader2 className="animate-spin" /> : <Radar />}
          {scanning ? "Scanning" : "Scan network"}
        </Button>
        <Button variant="outline" onClick={onAdd}>
          <Plus />
          Add by IP
        </Button>
      </div>
    </div>
  );
}
