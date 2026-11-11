import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// Mock the Tauri bridge: the frontend only knows the IPC contract.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import App from "./App";
import type { Device, Entity, EntityState } from "./ipc";

const device: Device = {
  id: "device-1",
  node_name: "eam-test-device",
  display_name: "EAM Test Device",
  area_id: null,
  group_ids: [],
  preferred_host: "10.60.109.61",
  preferred_port: 6053,
  api_encryption_secret_ref: null,
  connection_status: "online",
  last_seen_at: null,
  firmware_version: "2026.10.2",
  mac_address: null,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const switchEntity: Entity = {
  id: "entity-switch",
  device_id: "device-1",
  native_entity_key: "1040761112",
  object_id: null,
  name: "Test Switch",
  entity_type: "switch",
  unit: null,
  icon: null,
  state_class: null,
  device_class: null,
  capabilities: {},
  raw_metadata: {},
  availability: "available",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const numberEntity: Entity = {
  ...switchEntity,
  id: "entity-number",
  native_entity_key: "4181477727",
  name: "Test Number",
  entity_type: "number",
  capabilities: { min_value: 0, max_value: 100, step: 1 },
};

const offState: EntityState = {
  entity_id: "entity-switch",
  value: { type: "bool", value: false },
  raw_value: null,
  source_timestamp: null,
  received_at: "2026-01-01T00:00:00Z",
  quality: "good",
};

/** Route each IPC command to its fixture. */
function respond(command: string) {
  switch (command) {
    case "get_devices":
      return [device];
    case "get_entities":
      return [switchEntity, numberEntity];
    case "get_entity_states":
      return [offState];
    case "get_runtime_status":
      return {
        healthy: true,
        connected_devices: 1,
        total_entities: 2,
        memory_usage: 0,
        uptime_secs: 5,
        checked_at: "2026-01-01T00:00:00Z",
      };
    case "get_workspace_info":
      return { id: "w", name: "Default", path: "/tmp/eam.db", created_at: "" };
    default:
      return null;
  }
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation(async (command: string) =>
    respond(command),
  );
});

describe("App", () => {
  it("renders the runtime status line from IPC", async () => {
    render(<App />);
    await waitFor(() =>
      expect(screen.getByTestId("runtime-status").textContent).toContain(
        "1 device(s) connected",
      ),
    );
    expect(screen.queryByText("Loading devices…")).toBeNull();
  });

  it("lists devices with their status", async () => {
    render(<App />);
    await waitFor(() =>
      expect(screen.getByText("EAM Test Device")).toBeDefined(),
    );
    expect(screen.getByText("eam-test-device")).toBeDefined();
    expect(screen.getByText("online")).toBeDefined();
  });

  it("opens a device and shows its entities", async () => {
    const user = userEvent.setup();
    render(<App />);
    await waitFor(() => user.click(screen.getByText("Open")));
    await waitFor(() => {
      expect(screen.getByText("Test Switch")).toBeDefined();
      expect(screen.getByText("Test Number")).toBeDefined();
    });
  });

  it("sends a switch command with the toggled value", async () => {
    const user = userEvent.setup();
    render(<App />);
    await waitFor(() => user.click(screen.getByText("Open")));

    const toggle = await screen.findByText("Toggle");
    await user.click(toggle);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("control_switch", {
        entityId: "entity-switch",
        on: true, // current state is OFF, so the toggle sends ON
      });
    }, { timeout: 3000 });
  });

  it("sends a number command with the slider value", async () => {
    const user = userEvent.setup();
    render(<App />);
    await waitFor(() => user.click(screen.getByText("Open")));

    const set = await screen.findByText("Set");
    await user.click(set);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("set_number", {
        entityId: "entity-number",
        value: 0,
      });
    });
  });
});
