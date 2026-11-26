import { useEffect, useState, type FormEvent } from "react";
import { Loader2, Plus } from "lucide-react";
import { toast } from "sonner";
import { api, type Config } from "@/lib/api";
import { errorText } from "@/lib/format";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface AddConsoleDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved: (config: Config) => void;
}

function isIpv4(value: string): boolean {
  const parts = value.split(".");
  if (parts.length !== 4) return false;
  return parts.every(
    (part) => /^\d{1,3}$/.test(part) && Number(part) <= 255,
  );
}

function looksLikeAddress(value: string): boolean {
  if (isIpv4(value)) return true;
  return /^[a-zA-Z0-9][a-zA-Z0-9.-]{0,252}$/.test(value);
}

export function AddConsoleDialog({
  open,
  onOpenChange,
  onSaved,
}: AddConsoleDialogProps) {
  const [name, setName] = useState("");
  const [address, setAddress] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) {
      setName("");
      setAddress("");
      setError(null);
    }
  }, [open]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const cleanName = name.trim() || "PlayStation 5";
    const cleanAddress = address.trim();

    if (!looksLikeAddress(cleanAddress)) {
      setError("Enter an address like 192.168.1.50.");
      return;
    }

    setSaving(true);
    setError(null);
    try {
      const config = await api.upsertHost({
        name: cleanName,
        address: cleanAddress,
        manual: true,
      });
      onSaved(config);
      toast.success("Console added", { description: cleanName });
      onOpenChange(false);
    } catch (err) {
      setError(errorText(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[460px]">
        <form onSubmit={submit} className="space-y-4">
          <DialogHeader>
            <DialogTitle>Add a console</DialogTitle>
            <DialogDescription>
              Type the PS5 address if the scan cannot find it.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-3">
            <div className="space-y-1.5">
              <Label htmlFor="console-name">Name (optional)</Label>
              <Input
                id="console-name"
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="Living Room PS5"
                maxLength={64}
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="console-address">Address</Label>
              <Input
                id="console-address"
                value={address}
                onChange={(event) => setAddress(event.target.value)}
                placeholder="192.168.1.50"
                spellCheck={false}
                autoComplete="off"
                autoFocus
                className="font-mono"
              />
              {error ? (
                <p className="text-xs text-destructive">{error}</p>
              ) : null}
            </div>

            <div className="rounded-lg border border-white/5 bg-white/[0.03] p-3 text-xs text-muted-foreground">
              <p className="font-medium text-foreground">Where to find it</p>
              <p className="mt-1 leading-relaxed">
                On the PS5, open{" "}
                <span className="text-foreground">
                  Settings → Network → View Connection Status
                </span>
                . The IP address is on that screen.
              </p>
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={saving}>
              {saving ? <Loader2 className="animate-spin" /> : <Plus />}
              Add console
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
