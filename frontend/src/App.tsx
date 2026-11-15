import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { ProtectedRoute } from "@/components/ProtectedRoute";
import { AppLayout } from "@/components/AppLayout";
import { HomePage } from "@/pages/HomePage";
import { LoginPage } from "@/pages/LoginPage";
import { BookDetailPage } from "@/pages/BookDetailPage";
import { BookListPage } from "@/pages/BookListPage";
import { SearchPage } from "@/pages/SearchPage";
import { AdminLibrariesPage } from "@/pages/AdminLibrariesPage";
import { AdminUsersPage } from "@/pages/AdminUsersPage";
import { AdminKoreaderPage } from "@/pages/AdminKoreaderPage";
import { AdminBookOrbitPage } from "@/pages/AdminBookOrbitPage";
import { AdminMaintenancePage } from "@/pages/AdminMaintenancePage";
import { ReadingHistoryPage } from "@/pages/ReadingHistoryPage";
import { DuplicatesPage } from "@/pages/DuplicatesPage";

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
