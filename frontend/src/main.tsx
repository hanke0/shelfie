import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { ConfirmProvider } from "@/context/ConfirmContext";
import { LibraryProvider } from "@/context/LibraryContext";
import "./styles/global.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      staleTime: 30_000,
    },
  },
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <LibraryProvider>
        <ConfirmProvider>
          <App />
        </ConfirmProvider>
      </LibraryProvider>
    </QueryClientProvider>
  </StrictMode>,
);
