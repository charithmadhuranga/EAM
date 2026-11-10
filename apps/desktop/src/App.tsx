import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Device, DiscoveredDevice, RuntimeHealth } from "./ipc";

/**
 * Presentation layer only: all data arrives through Tauri IPC commands
 * backed by the Rust application layer (see AGENTS.md conventions).
 */
export default function App() {
  const [health, setHealth] = useState<RuntimeHealth | null>(null);
  const [devices, setDevices] = useState<Device[]>([]);
  const [discovered, setDiscovered] = useState<DiscoveredDevice[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [discovering, setDiscovering] = useState(false);

  useEffect(() => {
    invoke<RuntimeHealth>("get_runtime_status")
      .then(setHealth)
      .catch((e) => setError(String(e)));
    invoke<Device[]>("get_devices")
      .then(setDevices)
      .catch((e) => setError(String(e)));
  }, []);

  async function discover() {
    setDiscovering(true);
    setError(null);
    try {
      setDiscovered(await invoke<DiscoveredDevice[]>("discover_devices"));
    } catch (e) {
      setError(String(e));
    } finally {
      setDiscovering(false);
    }
  }

  return (
    <main style={{ fontFamily: "system-ui", padding: "2rem", maxWidth: 720 }}>
      <h1>ESPHome Automation Manager</h1>

      {error && <p style={{ color: "crimson" }}>Error: {error}</p>}

      <section>
        <h2>Runtime</h2>
        <p data-testid="runtime-status">
          {health
            ? `${health.healthy ? "Healthy" : "Degraded"} — ${health.connected_devices} connected, ${health.total_entities} entities`
            : "Loading…"}
        </p>
      </section>

      <section>
        <h2>Devices</h2>
        <button onClick={discover} disabled={discovering}>
          {discovering ? "Discovering…" : "Discover devices"}
        </button>
        <ul>
          {devices.map((d) => (
            <li key={d.id}>{d.display_name}</li>
          ))}
          {discovered.map((d) => (
            <li key={`${d.host}:${d.port}`}>
              {d.friendly_name ?? d.name} — {d.host}:{d.port}
            </li>
          ))}
          {devices.length === 0 && discovered.length === 0 && (
            <li>No devices yet</li>
          )}
        </ul>
      </section>
    </main>
  );
}
