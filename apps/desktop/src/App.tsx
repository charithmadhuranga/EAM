import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RuntimeHealth, WorkspaceInfo } from "./ipc";
import { FleetProvider } from "./stores/fleetStore";
import { DeviceList } from "./features/devices/DeviceList";

/**
 * Presentation layer only: all data arrives through Tauri IPC commands
 * backed by the Rust application layer (see AGENTS.md conventions).
 */
export default function App() {
  const [health, setHealth] = useState<RuntimeHealth | null>(null);
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        setHealth(await invoke<RuntimeHealth>("get_runtime_status"));
        setWorkspace(await invoke<WorkspaceInfo>("get_workspace_info"));
      } catch (e) {
        setError(String(e));
      }
    })();
  }, []);

  return (
    <FleetProvider>
      <main style={{ fontFamily: "system-ui", padding: "2rem", maxWidth: 960 }}>
        <h1>ESPHome Automation Manager</h1>

        {error && <p style={{ color: "crimson" }}>Error: {error}</p>}

        <section>
          <p data-testid="runtime-status">
            {health
              ? `${health.healthy ? "Healthy" : "Degraded"} — ${health.connected_devices} device(s) connected, ${health.total_entities} entities, uptime ${health.uptime_secs}s`
              : "Loading runtime…"}
            {workspace ? ` · ${workspace.name} (${workspace.path})` : ""}
          </p>
        </section>

        <DeviceList />
      </main>
    </FleetProvider>
  );
}
