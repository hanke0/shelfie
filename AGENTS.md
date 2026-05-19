# Shelfie — Agent Guide

## 项目简介

Shelfie 是图书管理 Web 应用：用户上传 PDF/EPUB/MOBI，按分类存于图书馆目录；每本书配套 `metadata.json` 与封面图；SQLite 存索引与权限；支持文件夹扫描同步。

## 四步路线图

| 步骤 | 状态 | 内容 |
|------|------|------|
| 1 | 完成 | Monorepo 骨架、health、空路由 |
| 2 | 完成 | OpenAPI 契约、migrations、stub handlers |
| 3 | 完成 | JWT、权限、CRUD、upload、refresh sync |
| 4 | 完成 | React 首页/详情/管理台 |

## 开发命令

```bash
# 根目录（Rust workspace + npm workspaces）
cargo run -p shelfie-backend
cargo test --workspace
npm install
npm run dev:frontend
npm run build

# 导出 OpenAPI → 前端 codegen
./scripts/export-openapi.sh
npm run codegen
```

## 目录约定

- `backend/src/domain/` — 业务逻辑，**不依赖** axum
- `backend/src/api/` — HTTP 层，薄 handler
- `backend/src/infra/` — DB、文件系统、metadata 读写
- 权限校验在 `api/middleware/`

## 文件布局（不可破坏）

```
{library_root}/{category}/
  {书名}_{作者}.pdf|epub|mobi
  {书名}_{作者}.json      # metadata
  {书名}_{作者}.jpg|png   # 封面
```

同一分类下所有图书平铺存放，不再为每本书单独建子目录。DB 仍用 UUID 作为 `book_id`；文件名由书名与作者拼接。

## OpenAPI 工作流

1. 修改 `backend/src/openapi.rs` 与各 handler 的 `utoipa` 注解
2. `./scripts/export-openapi.sh` → 更新 `openapi/openapi.json`
3. `cd frontend && npm run codegen`

### API 变更检查清单

- [ ] 已运行 `export-openapi.sh`
- [ ] 已运行 `npm run codegen`
- [ ] migrations 与 schema 一致
- [ ] Swagger UI `/swagger-ui` 可加载

## 环境变量

| 变量 | 说明 |
|------|------|
| `DATABASE_URL` | SQLite 连接串 |
| `DATA_ROOT` | 图书馆根目录父路径 |
| `JWT_SECRET` | JWT 签名密钥 |
| `PORT` | 监听端口，默认 8080 |

## Refresh 同步策略

- FS 有、DB 无 → INSERT（从 `metadata.json`）
- DB 有、FS 无 → 标记 `orphan`（不自动删 DB）
- 不一致 → 默认以 FS `metadata.json` 为准（`?prefer=db` 可选）

## 禁止提交

- `.env`、真实 `data/`、`.db` 文件
- `frontend/src/api/generated/`（由 codegen 生成）

## 测试

- `cargo test` — 后端单元/集成测试
- 前端 vitest 占位，后续补充
