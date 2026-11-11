import { useEffect, useState } from "react";
import type { EntityValue } from "../../ipc";
import { useFleet } from "../../stores/fleetStore";

/** One device with its entities and live controls. */
export function DeviceDetail({
  deviceId,
  onBack,
}: {
  deviceId: string;
  onBack(): void;
}) {
  const { devices, entities, states, refresh, unregister } = useFleet();
  const device = devices.find((d) => d.id === deviceId);
  const list = entities[deviceId] ?? [];

  if (!device) {
    return (
      <section>
        <button onClick={onBack}>← Devices</button>
        <p>Device not found.</p>
      </section>
    );
  }

  const controllable = list.filter((e) =>
    ["switch", "number", "light", "button", "select", "text"].includes(e.entity_type),
  );
  const readOnly = list.filter(
    (e) => !["switch", "number", "light", "button", "select", "text"].includes(e.entity_type),
  );

  return (
    <section>
      <header style={styles.header}>
        <div>
          <button onClick={onBack}>← Devices</button>{" "}
          <strong>{device.display_name}</strong>
          <span style={styles.muted}>
            {" "}
            {device.node_name} · {device.preferred_host}
          </span>
        </div>
        <div style={styles.actions}>
          <RefreshButton onRefresh={refresh} />
          <button onClick={() => unregister(deviceId)}>Forget</button>
        </div>
      </header>

      {controllable.length > 0 && <h3>Controls</h3>}
      <ul style={styles.list}>
        {controllable.map((entity) => (
          <EntityControl key={entity.id} entity={entity} />
        ))}
      </ul>

      {readOnly.length > 0 && <h3>Sensors</h3>}
      <ul style={styles.list}>
        {readOnly.map((entity) => (
          <li key={entity.id} style={styles.row}>
            <span>{entity.name}</span>
            <StateBadge value={states[entity.id]?.value} unit={entity.unit} />
          </li>
        ))}
      </ul>

      {list.length === 0 && <p>No entities reported by this device.</p>}
    </section>
  );
}

/** Dispatch to the control matching the entity type. */
function EntityControl({ entity }: { entity: import("../../ipc").Entity }) {
  switch (entity.entity_type) {
    case "switch":
      return <SwitchControl entity={entity} />;
    case "number":
      return <NumberControl entity={entity} />;
    case "light":
      return <SwitchControl entity={entity} label="On/Off" />;
    default:
      return (
        <li style={styles.row}>
          <span>{entity.name}</span>
          <span style={styles.muted}>{entity.entity_type} (control coming)</span>
        </li>
      );
  }
}

/** On/off control backed by the Rust command service. */
function SwitchControl({
  entity,
  label = "Toggle",
}: {
  entity: import("../../ipc").Entity;
  label?: string;
}) {
  const { states, toggleSwitch } = useFleet();
  const value = states[entity.id]?.value;
  const on = value?.type === "bool" && value.value === true;
  const [pending, setPending] = useState(false);

  return (
    <li style={styles.row}>
      <span>{entity.name}</span>
      <div style={styles.actions}>
        <span data-testid={`switch-state-${entity.id}`}>
          {on ? "ON" : "OFF"}
        </span>
        <button
          disabled={pending}
          onClick={async () => {
            setPending(true);
            try {
              await toggleSwitch(entity);
            } finally {
              setPending(false);
            }
          }}
        >
          {label}
        </button>
      </div>
    </li>
  );
}

/** Numeric control with min/max from entity capabilities. */
function NumberControl({ entity }: { entity: import("../../ipc").Entity }) {
  const { states, setNumber } = useFleet();
  const current = numericValue(states[entity.id]?.value);
  const capabilities = entity.capabilities ?? {};
  const min = Number(capabilities.min_value ?? 0);
  const max = Number(capabilities.max_value ?? 100);
  const step = Number(capabilities.step ?? 1);
  const [value, setValue] = useState(() =>
    typeof current === "number" ? current : min,
  );
  const [pending, setPending] = useState(false);

  useEffect(() => {
    if (typeof current === "number") setValue(current);
  }, [current]);

  return (
    <li style={styles.row}>
      <span>{entity.name}</span>
      <div style={styles.actions}>
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => setValue(Number(e.target.value))}
        />
        <span data-testid={`number-value-${entity.id}`}>
          {value}
          {entity.unit ? ` ${entity.unit}` : ""}
        </span>
        <button
          disabled={pending}
          onClick={async () => {
            setPending(true);
            try {
              await setNumber(entity, value);
            } finally {
              setPending(false);
            }
          }}
        >
          Set
        </button>
      </div>
    </li>
  );
}

/** Read a numeric state, if the value is numeric. */
function numericValue(value: EntityValue | undefined): number | undefined {
  return value && value.type === "number" ? value.value : undefined;
}

/** Renders a protocol value in a human-readable form. */
export function StateBadge({
  value,
  unit,
}: {
  value: import("../../ipc").EntityValue | undefined;
  unit?: string | null;
}) {
  if (!value) return <span style={styles.muted}>—</span>;
  if (value.type === "unknown") return <span style={styles.muted}>unknown</span>;
  const rendered = value.type === "bool" ? (value.value ? "on" : "off") : String(value.value);
  return (
    <span data-testid="state-value">
      {rendered}
      {unit ? ` ${unit}` : ""}
    </span>
  );
}

const styles: Record<string, React.CSSProperties> = {
  header: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    marginBottom: "1rem",
    gap: "1rem",
  },
  list: { listStyle: "none", padding: 0, margin: "0 0 1rem" },
  row: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    gap: "1rem",
    padding: "0.5rem 0",
    borderBottom: "1px solid #eee",
  },
  actions: { display: "flex", gap: "0.5rem", alignItems: "center" },
  muted: { color: "#666", fontSize: "0.85rem" },
};

/** Refresh button with a spinning label while working. */
function RefreshButton({ onRefresh }: { onRefresh(): Promise<void> }) {
  const [busy, setBusy] = useState(false);
  return (
    <button
      disabled={busy}
      onClick={async () => {
        setBusy(true);
        try {
          await onRefresh();
        } finally {
          setBusy(false);
        }
      }}
    >
      {busy ? "Refreshing…" : "Refresh"}
    </button>
  );
}
