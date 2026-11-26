import { invoke } from "@tauri-apps/api/core";

export type ConsoleState = "ready" | "standby" | "unknown";

export interface DiscoveredConsole {
  address: string;
  state: ConsoleState;
  host_id: string;
  name: string;
  host_type: string;
  system_version: string;
  protocol_version: string;
  request_port: number;
  running_app_title_id: string | null;
  running_app_name: string | null;
}

export interface SavedHost {
  id: string;
  name: string;
  address: string;
  regist_key: string | null;
  last_seen: number | null;
  manual: boolean;
}

export interface StreamSettings {
  resolution: string;
  fps: number;
  bitrate_mbps: number;
  codec: string;
  hdr: boolean;
}

export interface PerformanceSettings {
  prefer_nvidia: boolean;
  low_latency_present: boolean;
}

export interface Config {
  psn_account_id: string | null;
  hosts: SavedHost[];
  stream: StreamSettings;
  performance: PerformanceSettings;
}

export interface GpuInfo {
  name: string;
  driver: string;
  memory: string;
}

export interface SystemInfo {
  gpu: GpuInfo | null;
  ffmpeg: string | null;
  nvdec: boolean;
  os: string;
  arch: string;
}

export interface AppInfo {
  version: string;
  config_path: string;
  log_path: string;
  license: string;
  repository: string;
}

export const api = {
  getConfig: () => invoke<Config>("get_config"),

  saveConfig: (config: Config) => invoke<Config>("save_config", { config }),

  upsertHost: (args: {
    name: string;
    address: string;
    hostId?: string | null;
    registKey?: string | null;
    manual?: boolean;
  }) =>
    invoke<Config>("upsert_host", {
      name: args.name,
      address: args.address,
      hostId: args.hostId ?? null,
      registKey: args.registKey ?? null,
      manual: args.manual ?? true,
    }),

  removeHost: (id: string) => invoke<Config>("remove_host", { id }),

  discover: (waitMs = 3000, direct: string[] = []) =>
    invoke<DiscoveredConsole[]>("discover_consoles", { waitMs, direct }),

  wake: (address: string, registKey?: string | null) =>
    invoke<string>("wake_console", { address, registKey: registKey ?? null }),

  systemInfo: () => invoke<SystemInfo>("system_info"),

  appInfo: () => invoke<AppInfo>("app_info"),
};
