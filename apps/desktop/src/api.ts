import { invoke } from "@tauri-apps/api/core";
import type {
  Device,
  DiscoveredDevice,
  Entity,
  EntityState,
} from "./ipc";

/** Small typed wrapper around the Tauri IPC surface. */
export const api = {
  runtimeStatus: () => invoke<import("./ipc").RuntimeHealth>("get_runtime_status"),
  workspaceInfo: () => invoke<import("./ipc").WorkspaceInfo>("get_workspace_info"),

  devices: () => invoke<Device[]>("get_devices"),
  entities: (deviceId: string) => invoke<Entity[]>("get_entities", { deviceId }),
  entityStates: () => invoke<EntityState[]>("get_entity_states"),

  connect: (deviceId: string) => invoke<void>("connect_device", { deviceId }),
  disconnect: (deviceId: string) => invoke<void>("disconnect_device", { deviceId }),
  unregister: (deviceId: string) => invoke<void>("unregister_device", { deviceId }),

  scan: () => invoke<DiscoveredDevice[]>("scan_devices"),
  probe: (host: string) =>
    invoke<DiscoveredDevice>("probe_device", { host }),
  registerManual: (host: string, nodeName?: string, port?: number) =>
    invoke<Device>("register_device", { host, nodeName, port }),

  toggleSwitch: (entityId: string, on: boolean) =>
    invoke<void>("control_switch", { entityId, on }),
  setNumber: (entityId: string, value: number) =>
    invoke<void>("set_number", { entityId, value }),
};
