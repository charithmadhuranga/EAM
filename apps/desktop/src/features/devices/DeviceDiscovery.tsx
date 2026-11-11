import { useState } from "react";
import { api } from "../../api";
import type { DiscoveredDevice } from "../../ipc";
import { useFleet } from "../../stores/fleetStore";

/** Discovery modal: scans the subnet or adds a device by address. */
export function DeviceDiscovery({ onClose }: { onClose(): void }) {
  const { refresh } = useFleet();
  const [found, setFound] = useState<DiscoveredDevice[]>([]);
  const [scanning, setScanning] = useState(false);
  const [host, setHost] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function scan() {
    setScanning(true);
    setError(null);
    try {
      setFound(await api.scan());
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }

  async function addManual() {
    setBusy(true);
    setError(null);
    try {
      await api.registerManual(host.trim());
      await refresh();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function addDiscovered(device: DiscoveredDevice) {
    setBusy(true);
    try {
      await api.registerManual(device.host);
      await refresh();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div style={styles.backdrop} role="dialog" aria-modal="true">
      <div style={styles.modal}>
        <header style={styles.header}>
          <h2 style={{ margin: 0 }}>Discover devices</h2>
          <button onClick={onClose} aria-label="Close">✕</button>
        </header>

        <section style={styles.section}>
          <button onClick={scan} disabled={scanning}>
            {scanning ? "Scanning subnet…" : "Scan local subnet"}
          </button>
          {found.length === 0 && !scanning && (
            <p style={styles.hint}>
              Scans the local /24 with unicast mDNS probes. Devices are found
              even when multicast is filtered.
            </p>
          )}
          <ul style={styles.list}>
            {found.map((d) => (
              <li key={`${d.host}:${d.port}`} style={styles.listItem}>
                <span>
                  <strong>{d.friendly_name ?? d.name}</strong>
                  <span style={styles.muted}> {d.host}:{d.port}</span>
                </span>
                <button onClick={() => addDiscovered(d)} disabled={busy}>
                  Add
                </button>
              </li>
            ))}
          </ul>
        </section>

        <section style={styles.section}>
          <h3 style={{ margin: "0 0 0.5rem" }}>Add by address</h3>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <input
              value={host}
              onChange={(e) => setHost(e.target.value)}
              placeholder="10.60.109.61"
              style={{ flex: 1, padding: "0.4rem" }}
            />
            <button onClick={addManual} disabled={busy || !host.trim()}>
              Add
            </button>
          </div>
        </section>

        {error && <p style={{ color: "crimson" }}>{error}</p>}
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  backdrop: {
    position: "fixed",
    inset: 0,
    background: "rgba(0,0,0,0.4)",
    display: "grid",
    placeItems: "center",
    zIndex: 10,
  },
  modal: {
    background: "#fff",
    color: "#111",
    borderRadius: 8,
    padding: "1.25rem",
    width: "min(560px, 92vw)",
    maxHeight: "85vh",
    overflow: "auto",
  },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  section: { marginTop: "1rem" },
  hint: { color: "#666", fontSize: "0.85rem" },
  list: { listStyle: "none", padding: 0, margin: "0.5rem 0 0" },
  listItem: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "0.5rem 0",
    borderBottom: "1px solid #eee",
  },
  muted: { color: "#888", fontSize: "0.85rem" },
};
