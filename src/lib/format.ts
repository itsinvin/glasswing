import type { ConsoleState } from "@/lib/api";

export function stateLabel(state: ConsoleState | "offline"): string {
  switch (state) {
    case "ready":
      return "Ready";
    case "standby":
      return "Rest mode";
    case "offline":
      return "Offline";
    default:
      return "Unknown";
  }
}

export function timeAgo(unixSeconds: number | null | undefined): string {
  if (!unixSeconds) return "Never";

  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - unixSeconds);
  if (seconds < 60) return "Just now";

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hr ago`;

  const days = Math.floor(hours / 24);
  return `${days} day${days === 1 ? "" : "s"} ago`;
}

export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Something went wrong.";
}
