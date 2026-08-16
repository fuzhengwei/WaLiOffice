/**
 * dsh-persistence-sqlite: SQLite 持久化插件
 *
 * 将 DSH 的 session 事件持久化到 SQLite，与 Rust 后端共享数据库。
 *
 * 注意：当前版本使用 dsh-base 内置的 session-persistence-jsonl，
 * SQLite 持久化将在后续版本中实现（需要与 Rust 的 rusqlite 数据库共享）。
 *
 * 当前功能：
 * - 读取 Rust SQLite 数据库中的会话列表，供 DSH agent 使用
 * - 将 DSH agent 的 session 信息同步回 Rust 数据库
 */

import { Service } from '@deepseek-ai/cordis';

export default class PersistenceSQLite extends Service {
  static inject = ['sessions'];

  constructor(ctx: any, config: any) {
    super(ctx, 'persistenceSqlite');

    ctx.logger?.info?.('[dsh-persistence-sqlite] Initialized (using JSONL persistence from dsh-base)');

    // TODO: 实现与 Rust SQLite 的双向同步
    // 1. 读取 Rust 的 sessions 表获取历史会话
    // 2. DSH session 事件写入 Rust 的 messages 表
  }
}

export function apply(ctx: any, config: any) {
  ctx.plugin(PersistenceSQLite, config);
}
