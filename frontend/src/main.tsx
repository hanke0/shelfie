import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "./styles/global.css";
import App from "./App";
import { ConfirmProvider } from "@/context/ConfirmContext";
import { ToastProvider } from "@/context/ToastContext";
import { LibraryProvider } from "@/context/LibraryContext";

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
        <ToastProvider>
          <ConfirmProvider>
            <App />
          </ConfirmProvider>
        </ToastProvider>
      </LibraryProvider>
    </QueryClientProvider>
  </StrictMode>,
);
