# Shelfie — Agent Guide

## 项目简介

Shelfie 是图书管理 Web 应用：用户上传 PDF/EPUB/MOBI，按分类存于图书馆目录；每本书配套 `metadata.json` 与封面图；SQLite 存索引与权限；支持文件夹扫描同步。

## 四步路线图

| 步骤 | 状态 | 内容 |
|------|------|------|
| 1 | 完成 | Monorepo 骨架、health、空路由 |
| 2 | 完成 | OpenAPI 契约、migrations、stub handlers |
| 3 | 完成 | JWT、权限、CRUD、upload、维护任务（目录对比同步） |
| 4 | 完成 | React 首页/详情/管理台 |

## 开发命令

```bash
# 根目录（Rust workspace + npm workspaces）
cargo run -p shelfie-backend
cargo test --workspace
npm install
npm run dev:frontend
npm run build

# 单二进制：嵌入前端 dist，由后端托管静态资源
npm run build:bundle
cargo run -p shelfie-backend --features embed-frontend

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
| `HOST` | 监听地址，默认 `127.0.0.1`（局域网访问可设为 `0.0.0.0`） |
| `PORT` | 监听端口，默认 8080 |

## 系统维护与后台任务

- 通用长任务：`domain/tasks.rs`（`TaskManager`），记录存 `maintenance_tasks` 表；同一 `lock_key`（图书馆工具为 `library:{id}`）同时只允许一个 running 任务（部分唯一索引），冲突返回 409；取消为协作式（任务循环内检查 `TaskContext::is_cancelled`）；启动时残留 running 标记为 `interrupted`
- 工具注册：`domain/maintenance.rs`；新增工具需加 `MaintenanceTool` 枚举值、`describe`/`permission` 与分发分支
- 接口：`GET /maintenance/tools`、`GET|POST /maintenance/tasks`、`GET /maintenance/tasks/{id}`、`POST /maintenance/tasks/{id}/cancel`；可见性按图书馆编辑权限，破坏性工具需删除权限
- 图书馆对比（`domain/sync.rs`）：`library_scan` 只读出报告（DB 有 FS 无 / FS 有 DB 无 / 多余附属文件）；后续工具 `library_import_untracked`、`library_delete_untracked`、`library_purge_missing`、`library_clean_dangling`、`library_resync_metadata` 执行前重新扫描，`items` 中不再符合条件的项记为 skipped
- 哈希工具：`library_refresh_hashes`（全部重算）、`library_fill_missing_hashes`（仅缺 MD5 / partial MD5 / SHA-256 任一项的），写回 DB 与 metadata.json，文件缺失的图书不处理

## 禁止提交

- `.env`、真实 `data/`、`.db` 文件
- `frontend/src/api/generated/`（由 codegen 生成）

## 测试

- `cargo test` — 后端单元/集成测试
- `npm run test:frontend` — 前端 vitest（jsdom + Testing Library），测试文件与源码同目录 `*.test.ts(x)`

## Lint（改完代码必做）

Agent 完成代码修改后须跑 lint 并修到通过，再收尾：

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
```
