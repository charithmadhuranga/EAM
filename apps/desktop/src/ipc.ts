/**
 * IPC contract types mirrored from the Rust `eam-domain` crate.
 * These must stay in sync with `crates/eam-domain/src/{device,types}.rs`.
 */

export interface Device {
  id: string;
  node_name: string;
  display_name: string;
  area_id: string | null;
  group_ids: string[];
  preferred_host: string | null;
  preferred_port: number;
}

export interface DiscoveredDevice {
  name: string;
  host: string;
  port: number;
  mac_address: string | null;
  friendly_name: string | null;
  source: "mdns" | "manual" | "network_scan";
}

export interface RuntimeHealth {
  healthy: boolean;
  connected_devices: number;
  total_entities: number;
  memory_usage: number;
  uptime_secs: number;
  checked_at: string;
}
