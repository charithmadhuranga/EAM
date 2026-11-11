import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { api } from "../api";
import type { Device, Entity, EntityState } from "../ipc";

/** Live device + entity state, refreshed from the Rust runtime. */
interface FleetState {
  devices: Device[];
  entities: Record<string, Entity[]>;
  states: Record<string, EntityState>;
  loading: boolean;
  error: string | null;
  refresh(): Promise<void>;
  connect(id: string): Promise<void>;
  disconnect(id: string): Promise<void>;
  unregister(id: string): Promise<void>;
  toggleSwitch(entity: Entity): Promise<void>;
  setNumber(entity: Entity, value: number): Promise<void>;
}

const FleetContext = createContext<FleetState | null>(null);

/** Poll cadence for live state; the Rust side pushes but IPC is pull-based. */
/** Overridable in tests (`VITE_FLEET_POLL_MS`) to disable background polling. */
function pollIntervalMs(): number {
  const override = import.meta.env.VITE_FLEET_POLL_MS;
  const parsed = Number(override);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 2_000;
}

export function FleetProvider({ children }: { children: React.ReactNode }) {
  const [devices, setDevices] = useState<Device[]>([]);
  const [entities, setEntities] = useState<Record<string, Entity[]>>({});
  const [states, setStates] = useState<Record<string, EntityState>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadEntities = useCallback(async (list: Device[]) => {
    const map: Record<string, Entity[]> = {};
    await Promise.all(
      list.map(async (device) => {
        try {
          map[device.id] = await api.entities(device.id);
        } catch {
          map[device.id] = [];
        }
      }),
    );
    setEntities(map);
  }, []);

  const refresh = useCallback(async () => {
    try {
      const [list, current] = await Promise.all([
        api.devices(),
        api.entityStates(),
      ]);
      setDevices(list);
      setStates(
        Object.fromEntries(current.map((state) => [state.entity_id, state])),
      );
      await loadEntities(list);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [loadEntities]);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), pollIntervalMs());
    return () => window.clearInterval(timer);
  }, [refresh]);

  const value = useMemo<FleetState>(
    () => ({
      devices,
      entities,
      states,
      loading,
      error,
      refresh,
      async connect(id) {
        await api.connect(id);
        await refresh();
      },
      async disconnect(id) {
        await api.disconnect(id);
        await refresh();
      },
      async unregister(id) {
        await api.unregister(id);
        await refresh();
      },
      async toggleSwitch(entity) {
        // Toggle means "the opposite of what we last saw", so an unknown state
        // is treated as OFF and the command turns the entity ON.
        const current = states[entity.id]?.value;
        const isOn = current?.type === "bool" && current.value === true;
        await api.toggleSwitch(entity.id, !isOn);
        await refresh();
      },
      async setNumber(entity, value) {
        await api.setNumber(entity.id, value);
        await refresh();
      },
    }),
    [devices, entities, states, loading, error, refresh],
  );

  return <FleetContext.Provider value={value}>{children}</FleetContext.Provider>;
}

/** Access the fleet store.
 * @throws when used outside {@link FleetProvider}.
 */
export function useFleet(): FleetState {
  const ctx = useContext(FleetContext);
  if (!ctx) throw new Error("useFleet must be used inside <FleetProvider>");
  return ctx;
}
