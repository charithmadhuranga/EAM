import "@testing-library/jest-dom/vitest";

// Disable the fleet store's background polling so component tests are stable.
import.meta.env.VITE_FLEET_POLL_MS = "600000";
