import { lazy } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { ProtectedRoute } from "@/components/ProtectedRoute";
import { AppLayout } from "@/components/AppLayout";
import { HomePage } from "@/pages/HomePage";
import { LoginPage } from "@/pages/LoginPage";

const BookDetailPage = lazy(() =>
  import("@/pages/BookDetailPage").then((m) => ({ default: m.BookDetailPage })),
);
const BookListPage = lazy(() =>
  import("@/pages/BookListPage").then((m) => ({ default: m.BookListPage })),
);
const SearchPage = lazy(() =>
  import("@/pages/SearchPage").then((m) => ({ default: m.SearchPage })),
);
const AdminLibrariesPage = lazy(() =>
  import("@/pages/AdminLibrariesPage").then((m) => ({ default: m.AdminLibrariesPage })),
);
const AdminUsersPage = lazy(() =>
  import("@/pages/AdminUsersPage").then((m) => ({ default: m.AdminUsersPage })),
);
const AdminKoreaderPage = lazy(() =>
  import("@/pages/AdminKoreaderPage").then((m) => ({ default: m.AdminKoreaderPage })),
);
const AdminBookOrbitPage = lazy(() =>
  import("@/pages/AdminBookOrbitPage").then((m) => ({ default: m.AdminBookOrbitPage })),
);
const AdminMaintenancePage = lazy(() =>
  import("@/pages/AdminMaintenancePage").then((m) => ({ default: m.AdminMaintenancePage })),
);
const ReadingHistoryPage = lazy(() =>
  import("@/pages/ReadingHistoryPage").then((m) => ({ default: m.ReadingHistoryPage })),
);
const DuplicatesPage = lazy(() =>
  import("@/pages/DuplicatesPage").then((m) => ({ default: m.DuplicatesPage })),
);

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route
          element={
            <ProtectedRoute>
              <AppLayout />
            </ProtectedRoute>
          }
        >
          <Route path="/" element={<HomePage />} />
          <Route path="/books/:id" element={<BookDetailPage />} />
          <Route path="/books" element={<BookListPage />} />
          <Route path="/search" element={<SearchPage />} />
          <Route path="/admin/libraries" element={<AdminLibrariesPage />} />
          <Route path="/admin/users" element={<AdminUsersPage />} />
          <Route path="/admin/koreader" element={<AdminKoreaderPage />} />
          <Route path="/admin/bookorbit" element={<AdminBookOrbitPage />} />
          <Route path="/admin/maintenance" element={<AdminMaintenancePage />} />
          <Route path="/reading-history" element={<ReadingHistoryPage />} />
          <Route path="/duplicates" element={<DuplicatesPage />} />
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}
