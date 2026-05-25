import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { ProtectedRoute } from "@/components/ProtectedRoute";
import { HomePage } from "@/pages/HomePage";
import { LoginPage } from "@/pages/LoginPage";
import { BookDetailPage } from "@/pages/BookDetailPage";
import { BookListPage } from "@/pages/BookListPage";
import { SearchPage } from "@/pages/SearchPage";
import { AdminLibrariesPage } from "@/pages/AdminLibrariesPage";
import { AdminUsersPage } from "@/pages/AdminUsersPage";
import { AdminKoreaderPage } from "@/pages/AdminKoreaderPage";
import { ReadingHistoryPage } from "@/pages/ReadingHistoryPage";
import { DuplicatesPage } from "@/pages/DuplicatesPage";

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route
          path="/"
          element={
            <ProtectedRoute>
              <HomePage />
            </ProtectedRoute>
          }
        />
        <Route
          path="/books/:id"
          element={
            <ProtectedRoute>
              <BookDetailPage />
            </ProtectedRoute>
          }
        />
        <Route
          path="/books"
          element={
            <ProtectedRoute>
              <BookListPage />
            </ProtectedRoute>
          }
        />
        <Route
          path="/search"
          element={
            <ProtectedRoute>
              <SearchPage />
            </ProtectedRoute>
          }
        />
        <Route
          path="/admin/libraries"
          element={
            <ProtectedRoute>
              <AdminLibrariesPage />
            </ProtectedRoute>
          }
        />
        <Route
          path="/admin/users"
          element={
            <ProtectedRoute>
              <AdminUsersPage />
            </ProtectedRoute>
          }
        />
        <Route
          path="/admin/koreader"
          element={
            <ProtectedRoute>
              <AdminKoreaderPage />
            </ProtectedRoute>
          }
        />
        <Route
          path="/reading-history"
          element={
            <ProtectedRoute>
              <ReadingHistoryPage />
            </ProtectedRoute>
          }
        />
        <Route
          path="/duplicates"
          element={
            <ProtectedRoute>
              <DuplicatesPage />
            </ProtectedRoute>
          }
        />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}
