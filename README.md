# Shelfie

前后端分离的图书管理 Web 应用：Rust (Axum) 后端 + React (TypeScript) 前端。

## 快速开始

### 后端

```bash
cd backend
cp .env.example .env
cargo run
```

默认监听 `http://localhost:8080`，健康检查 `GET /api/v1/health`。

### 前端

```bash
cd frontend
cp .env.example .env
npm install
npm run codegen   # 需先 export openapi
npm run dev
```

### 导出 OpenAPI

```bash
./scripts/export-openapi.sh
cd frontend && npm run codegen
```

详见 [AGENTS.md](./AGENTS.md)。
