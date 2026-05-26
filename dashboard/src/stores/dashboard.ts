import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { TimeRange } from "@/types";

interface DashboardState {
  // Filters
  timeRange: TimeRange;
  environment: string;
  selectedProjects: string[];
  selectedIntelTypes: string[];

  // UI State
  autoRefresh: boolean;
  refreshInterval: number;
  sidebarOpen: boolean;
  theme: "dark" | "light";

  // Actions
  setTimeRange: (range: TimeRange) => void;
  setEnvironment: (env: string) => void;
  setSelectedProjects: (projects: string[]) => void;
  setSelectedIntelTypes: (types: string[]) => void;
  setAutoRefresh: (enabled: boolean) => void;
  setRefreshInterval: (interval: number) => void;
  toggleSidebar: () => void;
  setTheme: (theme: "dark" | "light") => void;
}

export const useDashboardStore = create<DashboardState>()(
  persist(
    (set) => ({
      // Initial state
      timeRange: "24h",
      environment: "all",
      selectedProjects: [],
      selectedIntelTypes: [],
      autoRefresh: true,
      refreshInterval: 30000, // 30 seconds
      sidebarOpen: true,
      theme: "dark",

      // Actions
      setTimeRange: (range) => set({ timeRange: range }),
      setEnvironment: (env) => set({ environment: env }),
      setSelectedProjects: (projects) => set({ selectedProjects: projects }),
      setSelectedIntelTypes: (types) => set({ selectedIntelTypes: types }),
      setAutoRefresh: (enabled) => set({ autoRefresh: enabled }),
      setRefreshInterval: (interval) => set({ refreshInterval: interval }),
      toggleSidebar: () =>
        set((state) => ({ sidebarOpen: !state.sidebarOpen })),
      setTheme: (theme) => set({ theme }),
    }),
    {
      name: "bridge-dashboard-storage",
      partialize: (state) => ({
        timeRange: state.timeRange,
        autoRefresh: state.autoRefresh,
        refreshInterval: state.refreshInterval,
        sidebarOpen: state.sidebarOpen,
        theme: state.theme,
      }),
    },
  ),
);

// WebSocket connection store
interface WebSocketState {
  connected: boolean;
  lastMessage: unknown | null;
  connect: () => void;
  disconnect: () => void;
  setMessage: (message: unknown) => void;
}

let ws: WebSocket | null = null;

export const useWebSocketStore = create<WebSocketState>((set) => ({
  connected: false,
  lastMessage: null,

  connect: () => {
    if (ws?.readyState === WebSocket.OPEN) return;

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

    ws.onopen = () => {
      set({ connected: true });
      console.log("WebSocket connected");
    };

    ws.onclose = () => {
      set({ connected: false });
      console.log("WebSocket disconnected");
      // Reconnect after 5 seconds
      setTimeout(() => {
        useWebSocketStore.getState().connect();
      }, 5000);
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        set({ lastMessage: data });
      } catch {
        console.error("Failed to parse WebSocket message");
      }
    };

    ws.onerror = (error) => {
      console.error("WebSocket error:", error);
    };
  },

  disconnect: () => {
    ws?.close();
    ws = null;
    set({ connected: false });
  },

  setMessage: (message) => set({ lastMessage: message }),
}));

// Intelligence query store
interface QueryState {
  query: string;
  results: unknown[];
  isSearching: boolean;
  setQuery: (query: string) => void;
  setResults: (results: unknown[]) => void;
  setIsSearching: (searching: boolean) => void;
  clearResults: () => void;
}

export const useQueryStore = create<QueryState>((set) => ({
  query: "",
  results: [],
  isSearching: false,

  setQuery: (query) => set({ query }),
  setResults: (results) => set({ results }),
  setIsSearching: (isSearching) => set({ isSearching }),
  clearResults: () => set({ results: [], query: "" }),
}));
