import { useState } from "react";
import { useFleet } from "../../stores/fleetStore";
import { DeviceDiscovery } from "./DeviceDiscovery";
import { DeviceDetail } from "./DeviceDetail";

/** Device fleet: the primary view of the app. */
export function DeviceList() {
  const { devices, loading, error, connect, disconnect } = useFleet();
  const [selected, setSelected] = useState<string | null>(null);
  const [showDiscovery, setShowDiscovery] = useState(false);

  if (selected) {
    return (
      <DeviceDetail
        deviceId={selected}
        onBack={() => setSelected(null)}
      />
    );
  }

  return (
    <section>
      <header style={styles.header}>
        <h2 style={{ margin: 0 }}>Devices</h2>
        <button onClick={() => setShowDiscovery(true)}>Add device</button>
      </header>

      {loading && <p>Loading devices…</p>}
      {error && <p style={{ color: "crimson" }}>Error: {error}</p>}
      {!loading && devices.length === 0 && (
        <p>No devices yet. Add one to get started.</p>
      )}

      <ul style={styles.grid}>
        {devices.map((device) => (
          <li key={device.id} style={styles.card}>
            <div>
              <strong>{device.display_name}</strong>
              <div style={styles.muted}>{device.node_name}</div>
              <div style={styles.muted}>
                {device.preferred_host}:{device.preferred_port}
              </div>
              <div style={styles.muted}>
                {device.firmware_version ?? "unknown firmware"}
              </div>
            </div>
            <div style={styles.actions}>
              <StatusBadge status={device.connection_status} />
              <button onClick={() => setSelected(device.id)}>Open</button>
              {device.connection_status === "online" ? (
                <button onClick={() => disconnect(device.id)}>Disconnect</button>
              ) : (
                <button onClick={() => connect(device.id)}>Connect</button>
              )}
            </div>
          </li>
        ))}
      </ul>

      {showDiscovery && (
        <DeviceDiscovery onClose={() => setShowDiscovery(false)} />
      )}
    </section>
  );
}

/** Connection status pill. */
export function StatusBadge({ status }: { status: string }) {
  const color =
    status === "online"
      ? "#1a7f37"
      : status === "connecting"
        ? "#9a6700"
        : status === "error" || status === "authentication_failed"
          ? "#cf222e"
          : "#6e7781";
  return (
    <span
      style={{
        color,
        border: `1px solid ${color}`,
        borderRadius: 999,
        padding: "0.1rem 0.6rem",
        fontSize: "0.8rem",
        textTransform: "capitalize",
      }}
    >
      {status.replace("_", " ")}
    </span>
  );
}


const styles: Record<string, React.CSSProperties> = {
  header: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    marginBottom: "1rem",
  },
  grid: {
    listStyle: "none",
    padding: 0,
    margin: 0,
    display: "grid",
    gap: "0.75rem",
    gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))",
  },
  card: {
    border: "1px solid #ddd",
    borderRadius: 8,
    padding: "1rem",
    display: "flex",
    flexDirection: "column",
    gap: "0.75rem",
    justifyContent: "space-between",
  },
  actions: { display: "flex", gap: "0.5rem", alignItems: "center" },
  muted: { color: "#666", fontSize: "0.85rem" },
};
