/**
 * IPC contract types mirrored from the Rust crates.
 * They must stay in sync with:
 *  - crates/eam-domain/src/{device,entity,types}.rs
 *  - crates/eam-esphome-api/src/client.rs
 *  - apps/desktop/src-tauri/src/commands/device.rs
 */

export type ConnectionStatus =
  | "online"
  | "connecting"
  | "offline"
  | "authentication_failed"
  | "error"
  | "disabled"
  | "unknown";

export interface Device {
  id: string;
  node_name: string;
  display_name: string;
  area_id: string | null;
  group_ids: string[];
  preferred_host: string | null;
  preferred_port: number;
  api_encryption_secret_ref: string | null;
  connection_status: ConnectionStatus;
  last_seen_at: string | null;
  firmware_version: string | null;
  mac_address: string | null;
  created_at: string;
  updated_at: string;
}

export type EntityType =
  | "sensor"
  | "binary_sensor"
  | "switch"
  | "light"
  | "number"
  | "select"
  | "button"
  | "text_sensor"
  | "text"
  | "fan"
  | "cover"
  | "climate"
  | "lock"
  | "media_player"
  | "unknown";

export interface Entity {
  id: string;
  device_id: string;
  native_entity_key: string;
  object_id: string | null;
  name: string;
  entity_type: EntityType;
  unit: string | null;
  icon: string | null;
  state_class: string | null;
  device_class: string | null;
  capabilities: Record<string, unknown>;
  raw_metadata: Record<string, unknown>;
  availability: "available" | "unavailable" | "stale";
  created_at: string;
  updated_at: string;
}

/** `EntityValue` tagged union (serde: tag=type, content=value). */
export type EntityValue =
  | { type: "number"; value: number }
  | { type: "bool"; value: boolean }
  | { type: "text"; value: string }
  | { type: "json"; value: unknown }
  | { type: "unknown" };

export interface EntityState {
  entity_id: string;
  value: EntityValue;
  raw_value: unknown | null;
  source_timestamp: string | null;
  received_at: string;
  quality: "good" | "poor" | "unknown";
}

export interface RuntimeHealth {
  healthy: boolean;
  connected_devices: number;
  total_entities: number;
  memory_usage: number;
  uptime_secs: number;
  checked_at: string;
}

export interface WorkspaceInfo {
  id: string;
  name: string;
  path: string;
  created_at: string;
}

export type DiscoverySource = "mdns" | "manual" | "network_scan";

export interface DiscoveredDevice {
  name: string;
  host: string;
  port: number;
  mac_address: string | null;
  friendly_name: string | null;
  source: DiscoverySource;
}
