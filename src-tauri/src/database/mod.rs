use anyhow::Result;
use rusqlite::Connection;

/// 数据库初始化 SQL 脚本
const INIT_SQL: &str = r#"
    -- 项目�?
    CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        path TEXT NOT NULL UNIQUE,
        description TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_projects_path ON projects(path);

    -- 会话�?
    CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        name TEXT NOT NULL,
        expert_id TEXT,
        agent_id TEXT,
        agent_type TEXT NOT NULL,
        cli_session_id TEXT,
        cli_session_provider TEXT,
        status TEXT NOT NULL DEFAULT 'idle',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_id);

    CREATE TABLE IF NOT EXISTS session_runtime_bindings (
        session_id TEXT NOT NULL,
        runtime_key TEXT NOT NULL,
        external_session_id TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (session_id, runtime_key),
        FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_session_runtime_bindings_runtime
        ON session_runtime_bindings(runtime_key, updated_at DESC);

    -- 消息�?
    CREATE TABLE IF NOT EXISTS messages (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        role TEXT NOT NULL,
        content TEXT NOT NULL,
        attachments TEXT,
        status TEXT NOT NULL DEFAULT 'completed',
        tokens INTEGER,
        created_at TEXT NOT NULL,
        FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);

    -- 智能体配置表
    CREATE TABLE IF NOT EXISTS agents (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        type TEXT NOT NULL,
        mode TEXT NOT NULL DEFAULT 'cli',
        api_key TEXT,
        base_url TEXT,
        model TEXT,
        cli_path TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    -- AgentTeams 专家表
    CREATE TABLE IF NOT EXISTS agent_experts (
        id TEXT PRIMARY KEY,
        builtin_code TEXT UNIQUE,
        name TEXT NOT NULL,
        description TEXT,
        prompt TEXT NOT NULL,
        runtime_agent_id TEXT,
        default_model_id TEXT,
        category TEXT NOT NULL DEFAULT 'custom',
        tags TEXT NOT NULL DEFAULT '[]',
        recommended_scenes TEXT NOT NULL DEFAULT '[]',
        is_builtin INTEGER NOT NULL DEFAULT 0,
        is_enabled INTEGER NOT NULL DEFAULT 1,
        sort_order INTEGER NOT NULL DEFAULT 100,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (runtime_agent_id) REFERENCES agents(id) ON DELETE SET NULL
    );
    CREATE INDEX IF NOT EXISTS idx_agent_experts_runtime ON agent_experts(runtime_agent_id);
    CREATE INDEX IF NOT EXISTS idx_agent_experts_enabled_order ON agent_experts(is_enabled, sort_order, updated_at DESC);

    -- MCP 服务器配置表
    CREATE TABLE IF NOT EXISTS mcp_servers (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        command TEXT NOT NULL,
        args TEXT,
        env TEXT,
        enabled INTEGER NOT NULL DEFAULT 1,
        test_status TEXT,
        test_message TEXT,
        tool_count INTEGER,
        tested_at TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    -- Skills 配置表（从市场安装的 Skills�?
    CREATE TABLE IF NOT EXISTS skills (
        id TEXT PRIMARY KEY,
        skill_id TEXT,                      -- 市场 Skill ID
        name TEXT NOT NULL,
        description TEXT,
        file_name TEXT NOT NULL,            -- 文件�?
        path TEXT NOT NULL,                 -- 完整路径
        source_market TEXT,                 -- 来源市场名称
        cli_type TEXT NOT NULL,             -- 目标 CLI (claude, cursor, aider, windsurf)
        scope TEXT NOT NULL DEFAULT 'global', -- 安装范围 (global, project)
        project_path TEXT,                  -- 项目路径（如果是 project scope�?
        disabled INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_skills_path ON skills(path);
    CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);

    -- 会话 MCP 关联�?
    CREATE TABLE IF NOT EXISTS session_mcp (
        session_id TEXT NOT NULL,
        mcp_server_id TEXT NOT NULL,
        enabled INTEGER NOT NULL DEFAULT 1,
        PRIMARY KEY (session_id, mcp_server_id),
        FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
        FOREIGN KEY (mcp_server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE
    );

    -- 主题配置�?
    CREATE TABLE IF NOT EXISTS themes (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        colors_light TEXT NOT NULL,
        colors_dark TEXT NOT NULL,
        created_at TEXT NOT NULL
    );

    -- 应用设置�?
    CREATE TABLE IF NOT EXISTS app_settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    -- CLI 路径配置表（手动配置�?
    CREATE TABLE IF NOT EXISTS cli_paths (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        path TEXT NOT NULL,
        version TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_cli_paths_name ON cli_paths(name);

    -- 市场源配置表
    CREATE TABLE IF NOT EXISTS market_sources (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        type TEXT NOT NULL DEFAULT 'github',
        url_or_path TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'active',
        enabled INTEGER NOT NULL DEFAULT 1,
        last_synced_at TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_market_sources_name ON market_sources(name);

    -- 已安�?MCP 测试结果表（存储 CLI 配置文件中的 MCP 测试结果�?
    CREATE TABLE IF NOT EXISTS installed_mcp_test_results (
        id TEXT PRIMARY KEY,
        config_path TEXT NOT NULL,
        mcp_name TEXT NOT NULL,
        test_status TEXT NOT NULL,
        test_message TEXT,
        tool_count INTEGER,
        tested_at TEXT NOT NULL,
        UNIQUE(config_path, mcp_name)
    );
    CREATE INDEX IF NOT EXISTS idx_installed_mcp_test_results_lookup ON installed_mcp_test_results(config_path, mcp_name);

    -- MCP 安装历史�?
    CREATE TABLE IF NOT EXISTS mcp_install_history (
        id TEXT PRIMARY KEY,
        mcp_id TEXT NOT NULL,
        mcp_name TEXT NOT NULL,
        cli_path TEXT NOT NULL,
        config_path TEXT NOT NULL,
        backup_path TEXT,
        scope TEXT NOT NULL DEFAULT 'global',
        status TEXT NOT NULL DEFAULT 'completed',
        created_at TEXT NOT NULL,
        rolled_back_at TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_mcp_install_history_created ON mcp_install_history(created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_mcp_install_history_mcp ON mcp_install_history(mcp_name);

    -- SDK 智能�?MCP 配置�?
    CREATE TABLE IF NOT EXISTS agent_mcp_configs (
        id TEXT PRIMARY KEY,
        agent_id TEXT NOT NULL,
        name TEXT NOT NULL,
        transport_type TEXT NOT NULL DEFAULT 'stdio',
        command TEXT,
        args TEXT,
        env TEXT,
        url TEXT,
        headers TEXT,
        scope TEXT NOT NULL DEFAULT 'user',
        enabled INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_agent_mcp_configs_agent ON agent_mcp_configs(agent_id);

    -- SDK 智能�?Skills 配置�?
    CREATE TABLE IF NOT EXISTS agent_skills_configs (
        id TEXT PRIMARY KEY,
        agent_id TEXT NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        skill_path TEXT NOT NULL,
        scripts_path TEXT,
        references_path TEXT,
        assets_path TEXT,
        enabled INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_agent_skills_configs_agent ON agent_skills_configs(agent_id);

    -- SDK 智能�?Plugins 配置�?
    CREATE TABLE IF NOT EXISTS agent_plugins_configs (
        id TEXT PRIMARY KEY,
        agent_id TEXT NOT NULL,
        name TEXT NOT NULL,
        version TEXT,
        description TEXT,
        plugin_path TEXT NOT NULL,
        enabled INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_agent_plugins_configs_agent ON agent_plugins_configs(agent_id);

    -- Provider 配置�?(CC-Switch)
    CREATE TABLE IF NOT EXISTS provider_profiles (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        cli_type TEXT NOT NULL,
        is_active INTEGER NOT NULL DEFAULT 0,
        api_key TEXT,
        base_url TEXT,
        provider_name TEXT,
        main_model TEXT,
        reasoning_model TEXT,
        haiku_model TEXT,
        sonnet_default TEXT,
        opus_default TEXT,
        codex_model TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_provider_profiles_cli_type ON provider_profiles(cli_type);
    CREATE INDEX IF NOT EXISTS idx_provider_profiles_is_active ON provider_profiles(is_active);

    -- 计划�?(Plan Mode)
    CREATE TABLE IF NOT EXISTS plans (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        split_expert_id TEXT,
        split_agent_id TEXT,
        split_model_id TEXT,
        status TEXT NOT NULL DEFAULT 'draft',
        agent_team TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_plans_project ON plans(project_id);
    CREATE INDEX IF NOT EXISTS idx_plans_status ON plans(status);

    CREATE TABLE IF NOT EXISTS solo_runs (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        execution_path TEXT NOT NULL DEFAULT '',
        name TEXT NOT NULL,
        requirement TEXT NOT NULL,
        goal TEXT NOT NULL,
        participant_expert_ids_json TEXT NOT NULL DEFAULT '[]',
        coordinator_expert_id TEXT,
        coordinator_agent_id TEXT,
        coordinator_model_id TEXT,
        max_dispatch_depth INTEGER NOT NULL DEFAULT 3,
        current_depth INTEGER NOT NULL DEFAULT 0,
        current_step_id TEXT,
        status TEXT NOT NULL DEFAULT 'draft',
        execution_status TEXT NOT NULL DEFAULT 'idle',
        last_error TEXT,
        input_request_json TEXT,
        input_response_json TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        started_at TEXT,
        completed_at TEXT,
        stopped_at TEXT,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_solo_runs_project ON solo_runs(project_id);
    CREATE INDEX IF NOT EXISTS idx_solo_runs_status ON solo_runs(status);

    CREATE TABLE IF NOT EXISTS solo_steps (
        id TEXT PRIMARY KEY,
        run_id TEXT NOT NULL,
        step_ref TEXT NOT NULL,
        parent_step_ref TEXT,
        depth INTEGER NOT NULL DEFAULT 1,
        title TEXT NOT NULL,
        description TEXT,
        execution_prompt TEXT,
        selected_expert_id TEXT,
        status TEXT NOT NULL DEFAULT 'pending',
        summary TEXT,
        result_summary TEXT,
        result_files_json TEXT,
        fail_reason TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        started_at TEXT,
        completed_at TEXT,
        FOREIGN KEY (run_id) REFERENCES solo_runs(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_solo_steps_run ON solo_steps(run_id, created_at ASC);
    CREATE INDEX IF NOT EXISTS idx_solo_steps_status ON solo_steps(status);
    CREATE UNIQUE INDEX IF NOT EXISTS idx_solo_steps_run_ref ON solo_steps(run_id, step_ref);

    CREATE TABLE IF NOT EXISTS solo_logs (
        id TEXT PRIMARY KEY,
        run_id TEXT NOT NULL,
        step_id TEXT,
        scope TEXT NOT NULL,
        log_type TEXT NOT NULL,
        content TEXT NOT NULL,
        metadata TEXT,
        created_at TEXT NOT NULL,
        FOREIGN KEY (run_id) REFERENCES solo_runs(id) ON DELETE CASCADE,
        FOREIGN KEY (step_id) REFERENCES solo_steps(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_solo_logs_run ON solo_logs(run_id, created_at ASC);
    CREATE INDEX IF NOT EXISTS idx_solo_logs_step ON solo_logs(step_id, created_at ASC);

    CREATE TABLE IF NOT EXISTS solo_runtime_bindings (
        run_id TEXT NOT NULL,
        runtime_key TEXT NOT NULL,
        external_session_id TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (run_id, runtime_key),
        FOREIGN KEY (run_id) REFERENCES solo_runs(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_solo_runtime_bindings_runtime
        ON solo_runtime_bindings(runtime_key, updated_at DESC);

    -- 任务�?(Plan Mode)
    CREATE TABLE IF NOT EXISTS tasks (
        id TEXT PRIMARY KEY,
        plan_id TEXT NOT NULL,
        parent_id TEXT,
        title TEXT NOT NULL,
        description TEXT,
        status TEXT NOT NULL DEFAULT 'pending',
        priority TEXT NOT NULL DEFAULT 'medium',
        assignee TEXT,
        expert_id TEXT,
        agent_id TEXT,
        model_id TEXT,
        session_id TEXT,
        cli_session_provider TEXT,
        progress_file TEXT,
        dependencies TEXT,
        task_order INTEGER NOT NULL DEFAULT 0,
        last_result_status TEXT,
        last_result_summary TEXT,
        last_result_files TEXT,
        last_fail_reason TEXT,
        last_result_at TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE,
        FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_tasks_plan ON tasks(plan_id);
    CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_id);
    CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);

    CREATE TABLE IF NOT EXISTS task_runtime_bindings (
        task_id TEXT NOT NULL,
        runtime_key TEXT NOT NULL,
        external_session_id TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (task_id, runtime_key),
        FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_task_runtime_bindings_runtime
        ON task_runtime_bindings(runtime_key, updated_at DESC);

    -- 智能体模型配置表
    CREATE TABLE IF NOT EXISTS agent_models (
        id TEXT PRIMARY KEY,
        agent_id TEXT NOT NULL,
        model_id TEXT NOT NULL,
        display_name TEXT NOT NULL,
        is_builtin INTEGER DEFAULT 0,
        is_default INTEGER DEFAULT 0,
        sort_order INTEGER DEFAULT 0,
        enabled INTEGER DEFAULT 1,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_agent_models_agent ON agent_models(agent_id);

    -- 应用状��表（窗口状态恢复）
    CREATE TABLE IF NOT EXISTS app_state (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL,
        updated_at INTEGER DEFAULT (strftime('%s', 'now'))
    );

    -- 项目访问记录表（朢�近项目列表）
    CREATE TABLE IF NOT EXISTS project_access_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        project_id TEXT NOT NULL UNIQUE,
        last_accessed_at INTEGER NOT NULL,
        access_count INTEGER DEFAULT 1,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_project_access_log_time ON project_access_log(last_accessed_at DESC);

    -- 窗口会话锁定表（防止同会话多窗口�?
    CREATE TABLE IF NOT EXISTS window_session_locks (
        session_id TEXT PRIMARY KEY,
        window_label TEXT NOT NULL,
        locked_at INTEGER DEFAULT (strftime('%s', 'now'))
    );

    -- 任务拆分会话表（存储AI原始输出和解析状态）
    CREATE TABLE IF NOT EXISTS task_split_sessions (
        id TEXT PRIMARY KEY,
        plan_id TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'processing',
        raw_content TEXT,
        parsed_output TEXT,
        parse_error TEXT,
        granularity INTEGER DEFAULT 20,
        task_count_mode TEXT NOT NULL DEFAULT 'min',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_task_split_sessions_plan ON task_split_sessions(plan_id);

    -- 任务执行日志�?
    CREATE TABLE IF NOT EXISTS task_execution_logs (
        id TEXT PRIMARY KEY,
        task_id TEXT NOT NULL,
        log_type TEXT NOT NULL,
        content TEXT NOT NULL,
        metadata TEXT,
        created_at TEXT NOT NULL,
        FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_task_execution_logs_task ON task_execution_logs(task_id);
    CREATE INDEX IF NOT EXISTS idx_task_execution_logs_created ON task_execution_logs(created_at);

    -- Agent CLI 用量统计表
    CREATE TABLE IF NOT EXISTS agent_cli_usage_records (
        execution_id TEXT PRIMARY KEY,
        execution_mode TEXT NOT NULL,
        provider TEXT NOT NULL,
        agent_id TEXT,
        agent_name_snapshot TEXT,
        model_id TEXT,
        project_id TEXT,
        session_id TEXT,
        task_id TEXT,
        message_id TEXT,
        input_tokens INTEGER NOT NULL DEFAULT 0,
        output_tokens INTEGER NOT NULL DEFAULT 0,
        total_tokens INTEGER NOT NULL DEFAULT 0,
        call_count INTEGER NOT NULL DEFAULT 1,
        estimated_input_cost_usd REAL,
        estimated_output_cost_usd REAL,
        estimated_total_cost_usd REAL,
        pricing_status TEXT NOT NULL DEFAULT 'missing_usage',
        pricing_version TEXT NOT NULL,
        occurred_at TEXT NOT NULL,
        created_at TEXT NOT NULL,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL,
        FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL,
        FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
        FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE SET NULL,
        FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE SET NULL
    );
    CREATE INDEX IF NOT EXISTS idx_agent_cli_usage_occurred ON agent_cli_usage_records(occurred_at DESC);
    CREATE INDEX IF NOT EXISTS idx_agent_cli_usage_provider_time ON agent_cli_usage_records(provider, occurred_at DESC);
    CREATE INDEX IF NOT EXISTS idx_agent_cli_usage_agent_time ON agent_cli_usage_records(agent_id, occurred_at DESC);
    CREATE INDEX IF NOT EXISTS idx_agent_cli_usage_model_time ON agent_cli_usage_records(model_id, occurred_at DESC);
    CREATE INDEX IF NOT EXISTS idx_agent_cli_usage_mode_time ON agent_cli_usage_records(execution_mode, occurred_at DESC);

    -- 部门�?
    CREATE TABLE IF NOT EXISTS departments (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        parent_id TEXT,
        description TEXT,
        manager_name TEXT,
        sort_order INTEGER DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'active',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (parent_id) REFERENCES departments(id) ON DELETE SET NULL
    );
    CREATE INDEX IF NOT EXISTS idx_departments_parent ON departments(parent_id);
    CREATE INDEX IF NOT EXISTS idx_departments_name ON departments(name);
    CREATE INDEX IF NOT EXISTS idx_departments_status ON departments(status);

    -- 人员�?
    CREATE TABLE IF NOT EXISTS employees (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        employee_no TEXT NOT NULL UNIQUE,
        department_id TEXT,
        position TEXT,
        phone TEXT,
        email TEXT,
        hire_date TEXT,
        status TEXT NOT NULL DEFAULT 'active',
        avatar TEXT,
        remark TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (department_id) REFERENCES departments(id) ON DELETE SET NULL
    );
    CREATE INDEX IF NOT EXISTS idx_employees_department ON employees(department_id);
    CREATE INDEX IF NOT EXISTS idx_employees_employee_no ON employees(employee_no);
    CREATE INDEX IF NOT EXISTS idx_employees_name ON employees(name);
    CREATE INDEX IF NOT EXISTS idx_employees_status ON employees(status);

    -- 任务执行结果快照历史�?
    CREATE TABLE IF NOT EXISTS task_execution_results (
        id TEXT PRIMARY KEY,
        task_id TEXT NOT NULL,
        plan_id TEXT NOT NULL,
        task_title_snapshot TEXT NOT NULL,
        task_description_snapshot TEXT,
        result_status TEXT NOT NULL,
        result_summary TEXT,
        result_files TEXT,
        fail_reason TEXT,
        created_at TEXT NOT NULL,
        FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
        FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_task_execution_results_plan_created
        ON task_execution_results(plan_id, created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_task_execution_results_task_created
        ON task_execution_results(task_id, created_at DESC);

    -- 无人值守渠道配置
    CREATE TABLE IF NOT EXISTS unattended_channels (
        id TEXT PRIMARY KEY,
        channel_type TEXT NOT NULL,
        name TEXT NOT NULL,
        enabled INTEGER NOT NULL DEFAULT 1,
        default_project_id TEXT,
        default_agent_id TEXT,
        default_model_id TEXT,
        reply_style TEXT NOT NULL DEFAULT 'final_only',
        allow_all_senders INTEGER NOT NULL DEFAULT 1,
        future_auth_mode TEXT NOT NULL DEFAULT 'allow_all',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (default_project_id) REFERENCES projects(id) ON DELETE SET NULL,
        FOREIGN KEY (default_agent_id) REFERENCES agents(id) ON DELETE SET NULL
    );
    CREATE INDEX IF NOT EXISTS idx_unattended_channels_type ON unattended_channels(channel_type);
    CREATE INDEX IF NOT EXISTS idx_unattended_channels_enabled ON unattended_channels(enabled);

    -- 无人值守渠道账号
    CREATE TABLE IF NOT EXISTS unattended_channel_accounts (
        id TEXT PRIMARY KEY,
        channel_id TEXT NOT NULL,
        account_id TEXT NOT NULL,
        user_id TEXT,
        base_url TEXT NOT NULL,
        bot_token TEXT NOT NULL,
        sync_cursor TEXT,
        login_status TEXT NOT NULL DEFAULT 'connected',
        runtime_status TEXT NOT NULL DEFAULT 'idle',
        last_connected_at TEXT,
        last_error TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (channel_id) REFERENCES unattended_channels(id) ON DELETE CASCADE,
        UNIQUE(channel_id, account_id)
    );
    CREATE INDEX IF NOT EXISTS idx_unattended_accounts_channel ON unattended_channel_accounts(channel_id, updated_at DESC);
    CREATE INDEX IF NOT EXISTS idx_unattended_accounts_runtime ON unattended_channel_accounts(runtime_status);

    -- 无人值守远程线程
    CREATE TABLE IF NOT EXISTS unattended_threads (
        id TEXT PRIMARY KEY,
        channel_account_id TEXT NOT NULL,
        peer_id TEXT NOT NULL,
        peer_name_snapshot TEXT,
        session_id TEXT,
        active_project_id TEXT,
        active_agent_id TEXT,
        active_model_id TEXT,
        last_context_token TEXT,
        last_plan_id TEXT,
        last_task_id TEXT,
        last_message_at TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (channel_account_id) REFERENCES unattended_channel_accounts(id) ON DELETE CASCADE,
        FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL,
        FOREIGN KEY (active_project_id) REFERENCES projects(id) ON DELETE SET NULL,
        FOREIGN KEY (active_agent_id) REFERENCES agents(id) ON DELETE SET NULL,
        FOREIGN KEY (last_plan_id) REFERENCES plans(id) ON DELETE SET NULL,
        FOREIGN KEY (last_task_id) REFERENCES tasks(id) ON DELETE SET NULL,
        UNIQUE(channel_account_id, peer_id)
    );
    CREATE INDEX IF NOT EXISTS idx_unattended_threads_account ON unattended_threads(channel_account_id, updated_at DESC);
    CREATE INDEX IF NOT EXISTS idx_unattended_threads_peer ON unattended_threads(peer_id);

    -- 无人值守审计日志
    CREATE TABLE IF NOT EXISTS unattended_events (
        id TEXT PRIMARY KEY,
        channel_account_id TEXT,
        thread_id TEXT,
        direction TEXT NOT NULL,
        event_type TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'success',
        summary TEXT,
        payload_json TEXT,
        correlation_id TEXT,
        created_at TEXT NOT NULL,
        FOREIGN KEY (channel_account_id) REFERENCES unattended_channel_accounts(id) ON DELETE SET NULL,
        FOREIGN KEY (thread_id) REFERENCES unattended_threads(id) ON DELETE SET NULL
    );
    CREATE INDEX IF NOT EXISTS idx_unattended_events_account_created ON unattended_events(channel_account_id, created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_unattended_events_thread_created ON unattended_events(thread_id, created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_unattended_events_type_created ON unattended_events(event_type, created_at DESC);

    -- 记忆分类表（用于 Skills 式层级展示）
    CREATE TABLE IF NOT EXISTS memory_categories (
        id TEXT PRIMARY KEY,
        parent_id TEXT,
        name TEXT NOT NULL,
        icon TEXT,
        color TEXT,
        description TEXT,
        order_index INTEGER DEFAULT 0,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (parent_id) REFERENCES memory_categories(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_memory_categories_parent ON memory_categories(parent_id);

    -- 用户记忆�?
    CREATE TABLE IF NOT EXISTS user_memories (
        id TEXT PRIMARY KEY,
        session_id TEXT,
        category_id TEXT,
        title TEXT NOT NULL,
        content TEXT NOT NULL,
        compressed_content TEXT,
        is_compressed INTEGER DEFAULT 0,
        source_type TEXT DEFAULT 'auto',
        source_message_ids TEXT,
        tags TEXT,
        metadata TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
        FOREIGN KEY (category_id) REFERENCES memory_categories(id) ON DELETE SET NULL
    );
    CREATE INDEX IF NOT EXISTS idx_user_memories_session ON user_memories(session_id);
    CREATE INDEX IF NOT EXISTS idx_user_memories_category ON user_memories(category_id);
    CREATE INDEX IF NOT EXISTS idx_user_memories_source_type ON user_memories(source_type);

    -- 记忆压缩历史�?
    CREATE TABLE IF NOT EXISTS memory_compressions (
        id TEXT PRIMARY KEY,
        memory_id TEXT NOT NULL,
        original_content TEXT NOT NULL,
        compressed_content TEXT NOT NULL,
        compression_ratio REAL,
        model_id TEXT,
        created_at TEXT NOT NULL,
        FOREIGN KEY (memory_id) REFERENCES user_memories(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_memory_compressions_memory ON memory_compressions(memory_id);
"#;

fn table_has_column(conn: &Connection, table_name: &str, column_name: &str) -> Result<bool> {
    let pragma_sql = format!("PRAGMA table_info({})", table_name);
    let mut stmt = conn.prepare(&pragma_sql)?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column_name {
            return Ok(true);
        }
    }

    Ok(false)
}

/// 初始化数据库
pub fn init_database() -> Result<()> {
    // 获取持久化目�?
    let persistence_dir = crate::commands::get_persistence_dir_path()?;
    let db_path = persistence_dir.join("data").join("easy-agent.db");

    // 确保目录存在
    std::fs::create_dir_all(db_path.parent().unwrap())?;

    println!("Database path: {:?}", db_path);

    // 打开数据库连�?
    let conn = Connection::open(&db_path)?;

    // 启用 WAL 模式以支持并发读写，避免 "database is locked" 错误
    conn.execute_batch("PRAGMA journal_mode = WAL")?;
    // 启用外键约束（SQLite 默认不启用）
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // 执行初始�?SQL
    conn.execute_batch(INIT_SQL)?;

    // 执行迁移（忽略列已存在的错误�?
    // SQLite 不支�?IF NOT EXISTS 用于 ALTER TABLE ADD COLUMN
    // 扢�以我们需要单独执行每条语句并忽略错误
    let migrations = [
        "ALTER TABLE mcp_servers ADD COLUMN test_status TEXT",
        "ALTER TABLE mcp_servers ADD COLUMN test_message TEXT",
        "ALTER TABLE mcp_servers ADD COLUMN tool_count INTEGER",
        "ALTER TABLE mcp_servers ADD COLUMN tested_at TEXT",
        "ALTER TABLE mcp_servers ADD COLUMN server_type TEXT DEFAULT 'stdio'",
        "ALTER TABLE mcp_servers ADD COLUMN url TEXT",
        "ALTER TABLE mcp_servers ADD COLUMN headers TEXT",
        // sessions 表添�?pinned �?last_message 字段
        "ALTER TABLE sessions ADD COLUMN pinned INTEGER DEFAULT 0",
        "ALTER TABLE sessions ADD COLUMN last_message TEXT",
        "ALTER TABLE sessions ADD COLUMN error_message TEXT",
        "ALTER TABLE sessions ADD COLUMN expert_id TEXT",
        "ALTER TABLE sessions ADD COLUMN agent_id TEXT",
        "ALTER TABLE sessions ADD COLUMN cli_session_id TEXT",
        "ALTER TABLE sessions ADD COLUMN cli_session_provider TEXT",
        "ALTER TABLE unattended_channels ADD COLUMN default_model_id TEXT",
        "ALTER TABLE unattended_threads ADD COLUMN active_project_id TEXT",
    ];

    for migration in migrations {
        // 忽略"列已存在"错误
        if let Err(e) = conn.execute(migration, []) {
            let err_str = e.to_string();
            if !err_str.contains("duplicate column name") {
                println!("Migration warning: {}", e);
            }
        }
    }

    // agents 表添加测试相关字�?
    let agent_migrations = [
        "ALTER TABLE agents ADD COLUMN status TEXT DEFAULT 'offline'",
        "ALTER TABLE agents ADD COLUMN test_message TEXT",
        "ALTER TABLE agents ADD COLUMN tested_at TEXT",
    ];

    for migration in agent_migrations {
        if let Err(e) = conn.execute(migration, []) {
            let err_str = e.to_string();
            if !err_str.contains("duplicate column name") {
                println!("Agent migration warning: {}", e);
            }
        }
    }

    // agents 表添加统丢�智能体模型字�?
    // provider: 提供�?(claude/codex)
    // model_id: 模型ID
    // custom_model_enabled: 是否启用自定义模�?
    let unified_agent_migrations = [
        "ALTER TABLE agents ADD COLUMN provider TEXT",
        "ALTER TABLE agents ADD COLUMN model_id TEXT",
        "ALTER TABLE agents ADD COLUMN custom_model_enabled INTEGER DEFAULT 0",
    ];

    for migration in unified_agent_migrations {
        if let Err(e) = conn.execute(migration, []) {
            let err_str = e.to_string();
            if !err_str.contains("duplicate column name") {
                println!("Unified agent migration warning: {}", e);
            }
        }
    }

    if !table_has_column(&conn, "solo_runs", "participant_expert_ids_json")? {
        conn.execute(
            "ALTER TABLE solo_runs ADD COLUMN participant_expert_ids_json TEXT NOT NULL DEFAULT '[]'",
            [],
        )?;
    }

    if !table_has_column(&conn, "solo_runs", "execution_path")? {
        conn.execute(
            "ALTER TABLE solo_runs ADD COLUMN execution_path TEXT NOT NULL DEFAULT ''",
            [],
        )?;
        conn.execute(
            "UPDATE solo_runs SET execution_path = COALESCE((SELECT path FROM projects WHERE projects.id = solo_runs.project_id), '') WHERE execution_path = ''",
            [],
        )?;
    }

    // skills 表添加新字段（从市场安装�?skills�?
    let skills_migrations = [
        "ALTER TABLE skills ADD COLUMN skill_id TEXT",
        "ALTER TABLE skills ADD COLUMN file_name TEXT",
        "ALTER TABLE skills ADD COLUMN source_market TEXT",
        "ALTER TABLE skills ADD COLUMN cli_type TEXT",
        "ALTER TABLE skills ADD COLUMN scope TEXT DEFAULT 'global'",
        "ALTER TABLE skills ADD COLUMN project_path TEXT",
        "ALTER TABLE skills ADD COLUMN disabled INTEGER DEFAULT 0",
    ];

    for migration in skills_migrations {
        if let Err(e) = conn.execute(migration, []) {
            let err_str = e.to_string();
            if !err_str.contains("duplicate column name") {
                println!("Skills migration warning: {}", e);
            }
        }
    }

    // 创建 skills 表的索引（如果不存在�?
    let index_migrations = [
        "CREATE INDEX IF NOT EXISTS idx_skills_path ON skills(path)",
        "CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name)",
    ];

    for migration in index_migrations {
        if let Err(e) = conn.execute(migration, []) {
            println!("Skills index migration warning: {}", e);
        }
    }

    // mcp_install_history 表迁移（如果表不存在则创建）
    let history_table_sql = r#"
        CREATE TABLE IF NOT EXISTS mcp_install_history (
            id TEXT PRIMARY KEY,
            mcp_id TEXT NOT NULL,
            mcp_name TEXT NOT NULL,
            cli_path TEXT NOT NULL,
            config_path TEXT NOT NULL,
            backup_path TEXT,
            scope TEXT NOT NULL DEFAULT 'global',
            status TEXT NOT NULL DEFAULT 'completed',
            created_at TEXT NOT NULL,
            rolled_back_at TEXT
        )
    "#;
    if let Err(e) = conn.execute(history_table_sql, []) {
        println!("MCP install history table migration warning: {}", e);
    }

    // 创建索引
    let history_index_migrations = [
        "CREATE INDEX IF NOT EXISTS idx_mcp_install_history_created ON mcp_install_history(created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_mcp_install_history_mcp ON mcp_install_history(mcp_name)",
    ];
    for migration in history_index_migrations {
        if let Err(e) = conn.execute(migration, []) {
            println!("MCP install history index migration warning: {}", e);
        }
    }

    // messages 表添�?error_message 字段（用于存储发送失败的原因�?
    let message_migrations = [
        "ALTER TABLE messages ADD COLUMN attachments TEXT",
        "ALTER TABLE messages ADD COLUMN error_message TEXT",
        "ALTER TABLE messages ADD COLUMN tool_calls TEXT", // JSON string for tool calls
        "ALTER TABLE messages ADD COLUMN thinking TEXT",   // 思��内容（扩展思维模型�?
        "ALTER TABLE messages ADD COLUMN edit_traces TEXT",
        "ALTER TABLE messages ADD COLUMN runtime_notices TEXT",
        "ALTER TABLE messages ADD COLUMN compression_metadata TEXT",
    ];

    for migration in message_migrations {
        if let Err(e) = conn.execute(migration, []) {
            let err_str = e.to_string();
            if !err_str.contains("duplicate column name") {
                println!("Messages migration warning: {}", e);
            }
        }
    }

    // agent_models 表迁移（智能体模型配置表�?
    let agent_models_table_sql = r#"
        CREATE TABLE IF NOT EXISTS agent_models (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            model_id TEXT NOT NULL,
            display_name TEXT NOT NULL,
            is_builtin INTEGER DEFAULT 0,
            is_default INTEGER DEFAULT 0,
            sort_order INTEGER DEFAULT 0,
            enabled INTEGER DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
        )
    "#;
    if let Err(e) = conn.execute(agent_models_table_sql, []) {
        println!("Agent models table migration warning: {}", e);
    }

    // 创建索引
    let agent_models_index_sql =
        "CREATE INDEX IF NOT EXISTS idx_agent_models_agent ON agent_models(agent_id)";
    if let Err(e) = conn.execute(agent_models_index_sql, []) {
        println!("Agent models index migration warning: {}", e);
    }

    // agent_models 表添�?context_window 字段
    let agent_models_migrations =
        ["ALTER TABLE agent_models ADD COLUMN context_window INTEGER DEFAULT 128000"];

    for migration in agent_models_migrations {
        if let Err(e) = conn.execute(migration, []) {
            let err_str = e.to_string();
            if !err_str.contains("duplicate column name") {
                println!("Agent models migration warning: {}", e);
            }
        }
    }

    // plans 表添加新字段（任务拆分颗粒度、最大重试次数��执行状态��当前任务ID�?
    let plans_migrations = [
        "ALTER TABLE plans ADD COLUMN granularity INTEGER DEFAULT 20",
        "ALTER TABLE plans ADD COLUMN max_retry_count INTEGER DEFAULT 3",
        "ALTER TABLE plans ADD COLUMN execution_status TEXT DEFAULT 'idle'",
        "ALTER TABLE plans ADD COLUMN current_task_id TEXT",
        "ALTER TABLE plans ADD COLUMN split_agent_id TEXT",
        "ALTER TABLE plans ADD COLUMN split_model_id TEXT",
        "ALTER TABLE plans ADD COLUMN split_expert_id TEXT",
        "ALTER TABLE plans ADD COLUMN scheduled_at TEXT",
        "ALTER TABLE plans ADD COLUMN schedule_status TEXT DEFAULT 'none'",
        "ALTER TABLE plans ADD COLUMN split_mode TEXT DEFAULT 'ai'",
    ];

    for migration in plans_migrations {
        if let Err(e) = conn.execute(migration, []) {
            let err_str = e.to_string();
            if !err_str.contains("duplicate column name") {
                println!("Plans migration warning: {}", e);
            }
        }
    }

    // tasks 表添加新字段（重试计数��最大重试��错误信息��实现步骤��测试步骤��验收标准）
    let tasks_migrations = [
        "ALTER TABLE tasks ADD COLUMN agent_id TEXT",
        "ALTER TABLE tasks ADD COLUMN model_id TEXT",
        "ALTER TABLE tasks ADD COLUMN expert_id TEXT",
        "ALTER TABLE tasks ADD COLUMN cli_session_provider TEXT",
        "ALTER TABLE tasks ADD COLUMN retry_count INTEGER DEFAULT 0",
        "ALTER TABLE tasks ADD COLUMN max_retries INTEGER DEFAULT 3",
        "ALTER TABLE tasks ADD COLUMN error_message TEXT",
        "ALTER TABLE tasks ADD COLUMN implementation_steps TEXT",
        "ALTER TABLE tasks ADD COLUMN test_steps TEXT",
        "ALTER TABLE tasks ADD COLUMN acceptance_criteria TEXT",
        "ALTER TABLE tasks ADD COLUMN last_result_status TEXT",
        "ALTER TABLE tasks ADD COLUMN last_result_summary TEXT",
        "ALTER TABLE tasks ADD COLUMN last_result_files TEXT",
        "ALTER TABLE tasks ADD COLUMN last_fail_reason TEXT",
        "ALTER TABLE tasks ADD COLUMN last_result_at TEXT",
        "ALTER TABLE tasks ADD COLUMN block_reason TEXT",
        "ALTER TABLE tasks ADD COLUMN input_request TEXT",
        "ALTER TABLE tasks ADD COLUMN input_response TEXT",
    ];

    for migration in tasks_migrations {
        if let Err(e) = conn.execute(migration, []) {
            let err_str = e.to_string();
            if !err_str.contains("duplicate column name") {
                println!("Tasks migration warning: {}", e);
            }
        }
    }

    let runtime_binding_tables = [
        r#"
        CREATE TABLE IF NOT EXISTS session_runtime_bindings (
            session_id TEXT NOT NULL,
            runtime_key TEXT NOT NULL,
            external_session_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (session_id, runtime_key),
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_session_runtime_bindings_runtime ON session_runtime_bindings(runtime_key, updated_at DESC)",
        r#"
        CREATE TABLE IF NOT EXISTS task_runtime_bindings (
            task_id TEXT NOT NULL,
            runtime_key TEXT NOT NULL,
            external_session_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (task_id, runtime_key),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_task_runtime_bindings_runtime ON task_runtime_bindings(runtime_key, updated_at DESC)",
    ];
    for migration in runtime_binding_tables {
        if let Err(e) = conn.execute(migration, []) {
            println!("Runtime binding migration warning: {}", e);
        }
    }

    let runtime_binding_backfills = [
        r#"
        INSERT OR IGNORE INTO session_runtime_bindings (
            session_id,
            runtime_key,
            external_session_id,
            created_at,
            updated_at
        )
        SELECT
            id,
            cli_session_provider || '-cli',
            cli_session_id,
            updated_at,
            updated_at
        FROM sessions
        WHERE cli_session_id IS NOT NULL
          AND trim(cli_session_id) != ''
          AND cli_session_provider IS NOT NULL
          AND trim(cli_session_provider) != ''
        "#,
        r#"
        INSERT OR IGNORE INTO task_runtime_bindings (
            task_id,
            runtime_key,
            external_session_id,
            created_at,
            updated_at
        )
        SELECT
            id,
            cli_session_provider || '-cli',
            session_id,
            updated_at,
            updated_at
        FROM tasks
        WHERE session_id IS NOT NULL
          AND trim(session_id) != ''
          AND cli_session_provider IS NOT NULL
          AND trim(cli_session_provider) != ''
        "#,
    ];
    for migration in runtime_binding_backfills {
        if let Err(e) = conn.execute(migration, []) {
            println!("Runtime binding backfill warning: {}", e);
        }
    }

    let agent_experts_table_sql = r#"
        CREATE TABLE IF NOT EXISTS agent_experts (
            id TEXT PRIMARY KEY,
            builtin_code TEXT UNIQUE,
            name TEXT NOT NULL,
            description TEXT,
            prompt TEXT NOT NULL,
            runtime_agent_id TEXT,
            default_model_id TEXT,
            category TEXT NOT NULL DEFAULT 'custom',
            tags TEXT NOT NULL DEFAULT '[]',
            recommended_scenes TEXT NOT NULL DEFAULT '[]',
            is_builtin INTEGER NOT NULL DEFAULT 0,
            is_enabled INTEGER NOT NULL DEFAULT 1,
            sort_order INTEGER NOT NULL DEFAULT 100,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (runtime_agent_id) REFERENCES agents(id) ON DELETE SET NULL
        )
    "#;
    if let Err(e) = conn.execute(agent_experts_table_sql, []) {
        println!("Agent experts table migration warning: {}", e);
    }

    let agent_experts_index_migrations = [
        "CREATE INDEX IF NOT EXISTS idx_agent_experts_runtime ON agent_experts(runtime_agent_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_experts_enabled_order ON agent_experts(is_enabled, sort_order, updated_at DESC)",
    ];
    for migration in agent_experts_index_migrations {
        if let Err(e) = conn.execute(migration, []) {
            println!("Agent experts index migration warning: {}", e);
        }
    }

    // task_split_sessions 表（存储AI原始输出和解析状态）
    let task_split_sessions_table_sql = r#"
        CREATE TABLE IF NOT EXISTS task_split_sessions (
            id TEXT PRIMARY KEY,
            plan_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'processing',
            raw_content TEXT,
            parsed_output TEXT,
            parse_error TEXT,
            granularity INTEGER DEFAULT 20,
            task_count_mode TEXT NOT NULL DEFAULT 'min',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE
        )
    "#;
    if let Err(e) = conn.execute(task_split_sessions_table_sql, []) {
        println!("Task split sessions table migration warning: {}", e);
    }

    // 创建索引
    let task_split_sessions_index_sql =
        "CREATE INDEX IF NOT EXISTS idx_task_split_sessions_plan ON task_split_sessions(plan_id)";
    if let Err(e) = conn.execute(task_split_sessions_index_sql, []) {
        println!("Task split sessions index migration warning: {}", e);
    }

    let task_split_sessions_migrations = [
        "ALTER TABLE task_split_sessions ADD COLUMN execution_session_id TEXT",
        "ALTER TABLE task_split_sessions ADD COLUMN execution_request_json TEXT",
        "ALTER TABLE task_split_sessions ADD COLUMN llm_messages_json TEXT",
        "ALTER TABLE task_split_sessions ADD COLUMN messages_json TEXT",
        "ALTER TABLE task_split_sessions ADD COLUMN form_queue_json TEXT",
        "ALTER TABLE task_split_sessions ADD COLUMN current_form_index INTEGER",
        "ALTER TABLE task_split_sessions ADD COLUMN error_message TEXT",
        "ALTER TABLE task_split_sessions ADD COLUMN started_at TEXT",
        "ALTER TABLE task_split_sessions ADD COLUMN completed_at TEXT",
        "ALTER TABLE task_split_sessions ADD COLUMN stopped_at TEXT",
        "ALTER TABLE task_split_sessions ADD COLUMN task_count_mode TEXT NOT NULL DEFAULT 'min'",
    ];
    for migration in task_split_sessions_migrations {
        if let Err(e) = conn.execute(migration, []) {
            let err_str = e.to_string();
            if !err_str.contains("duplicate column name") {
                println!("Task split sessions migration warning: {}", e);
            }
        }
    }

    let plan_split_logs_table_sql = r#"
        CREATE TABLE IF NOT EXISTS plan_split_logs (
            id TEXT PRIMARY KEY,
            plan_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            log_type TEXT NOT NULL,
            content TEXT NOT NULL,
            metadata TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE
        )
    "#;
    if let Err(e) = conn.execute(plan_split_logs_table_sql, []) {
        println!("Plan split logs table migration warning: {}", e);
    }

    let plan_split_logs_indexes = [
        "CREATE INDEX IF NOT EXISTS idx_plan_split_logs_plan ON plan_split_logs(plan_id, created_at ASC)",
        "CREATE INDEX IF NOT EXISTS idx_plan_split_logs_session ON plan_split_logs(session_id, created_at ASC)",
    ];
    for migration in plan_split_logs_indexes {
        if let Err(e) = conn.execute(migration, []) {
            println!("Plan split logs index migration warning: {}", e);
        }
    }

    // task_execution_results 表（存储任务执行完成/失败后的结构化结果）
    let task_execution_results_table_sql = r#"
        CREATE TABLE IF NOT EXISTS task_execution_results (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            plan_id TEXT NOT NULL,
            task_title_snapshot TEXT NOT NULL,
            task_description_snapshot TEXT,
            result_status TEXT NOT NULL,
            result_summary TEXT,
            result_files TEXT,
            fail_reason TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE
        )
    "#;
    if let Err(e) = conn.execute(task_execution_results_table_sql, []) {
        println!("Task execution results table migration warning: {}", e);
    }

    let task_execution_results_indexes = [
        "CREATE INDEX IF NOT EXISTS idx_task_execution_results_plan_created ON task_execution_results(plan_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_task_execution_results_task_created ON task_execution_results(task_id, created_at DESC)",
    ];
    for migration in task_execution_results_indexes {
        if let Err(e) = conn.execute(migration, []) {
            println!("Task execution results index migration warning: {}", e);
        }
    }

    let agent_cli_usage_table_sql = r#"
        CREATE TABLE IF NOT EXISTS agent_cli_usage_records (
            execution_id TEXT PRIMARY KEY,
            execution_mode TEXT NOT NULL,
            provider TEXT NOT NULL,
            agent_id TEXT,
            agent_name_snapshot TEXT,
            model_id TEXT,
            project_id TEXT,
            session_id TEXT,
            task_id TEXT,
            message_id TEXT,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            total_tokens INTEGER NOT NULL DEFAULT 0,
            call_count INTEGER NOT NULL DEFAULT 1,
            estimated_input_cost_usd REAL,
            estimated_output_cost_usd REAL,
            estimated_total_cost_usd REAL,
            pricing_status TEXT NOT NULL DEFAULT 'missing_usage',
            pricing_version TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
            FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE SET NULL,
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE SET NULL
        )
    "#;
    if let Err(e) = conn.execute(agent_cli_usage_table_sql, []) {
        println!("Agent CLI usage table migration warning: {}", e);
    }

    let agent_cli_usage_indexes = [
        "CREATE INDEX IF NOT EXISTS idx_agent_cli_usage_occurred ON agent_cli_usage_records(occurred_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_agent_cli_usage_provider_time ON agent_cli_usage_records(provider, occurred_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_agent_cli_usage_agent_time ON agent_cli_usage_records(agent_id, occurred_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_agent_cli_usage_model_time ON agent_cli_usage_records(model_id, occurred_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_agent_cli_usage_mode_time ON agent_cli_usage_records(execution_mode, occurred_at DESC)",
    ];
    for migration in agent_cli_usage_indexes {
        if let Err(e) = conn.execute(migration, []) {
            println!("Agent CLI usage index migration warning: {}", e);
        }
    }

    // ==================== Memory domain rebuild ====================
    let cleanup_legacy_memory_tables = [
        "DROP TABLE IF EXISTS memory_compressions",
        "DROP TABLE IF EXISTS user_memories",
        "DROP TABLE IF EXISTS memory_categories",
    ];
    for migration in cleanup_legacy_memory_tables {
        if let Err(e) = conn.execute(migration, []) {
            println!("Legacy memory cleanup warning: {}", e);
        }
    }

    let needs_memory_rebuild = !table_has_column(&conn, "memory_libraries", "content_md")?;
    if needs_memory_rebuild {
        let rebuild_tables = [
            "DROP TABLE IF EXISTS memory_merge_runs",
            "DROP TABLE IF EXISTS raw_memory_records",
            "DROP TABLE IF EXISTS memory_items",
            "DROP TABLE IF EXISTS memory_extractions",
            "DROP TABLE IF EXISTS memory_records",
            "DROP TABLE IF EXISTS memory_libraries",
        ];
        for migration in rebuild_tables {
            if let Err(e) = conn.execute(migration, []) {
                println!("Memory rebuild cleanup warning: {}", e);
            }
        }
    }

    let memory_libraries_table_sql = r#"
        CREATE TABLE IF NOT EXISTS memory_libraries (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            content_md TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
    "#;
    if let Err(e) = conn.execute(memory_libraries_table_sql, []) {
        println!("Memory libraries table migration warning: {}", e);
    }

    let raw_memory_records_table_sql = r#"
        CREATE TABLE IF NOT EXISTS raw_memory_records (
            id TEXT PRIMARY KEY,
            session_id TEXT,
            project_id TEXT,
            message_id TEXT UNIQUE,
            content TEXT NOT NULL,
            source_role TEXT NOT NULL DEFAULT 'user',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        )
    "#;
    if let Err(e) = conn.execute(raw_memory_records_table_sql, []) {
        println!("Raw memory records table migration warning: {}", e);
    }

    let memory_merge_runs_table_sql = r#"
        CREATE TABLE IF NOT EXISTS memory_merge_runs (
            id TEXT PRIMARY KEY,
            library_id TEXT NOT NULL,
            source_record_ids TEXT NOT NULL,
            source_record_count INTEGER NOT NULL DEFAULT 0,
            previous_content_md TEXT NOT NULL DEFAULT '',
            merged_content_md TEXT NOT NULL DEFAULT '',
            agent_id TEXT,
            model_id TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (library_id) REFERENCES memory_libraries(id) ON DELETE CASCADE
        )
    "#;
    if let Err(e) = conn.execute(memory_merge_runs_table_sql, []) {
        println!("Memory merge runs table migration warning: {}", e);
    }

    let project_memory_libraries_table_sql = r#"
        CREATE TABLE IF NOT EXISTS project_memory_libraries (
            project_id TEXT NOT NULL,
            library_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (project_id, library_id),
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
            FOREIGN KEY (library_id) REFERENCES memory_libraries(id) ON DELETE CASCADE
        )
    "#;
    if let Err(e) = conn.execute(project_memory_libraries_table_sql, []) {
        println!("Project memory libraries table migration warning: {}", e);
    }

    let memory_library_chunks_table_sql = r#"
        CREATE TABLE IF NOT EXISTS memory_library_chunks (
            id TEXT PRIMARY KEY,
            library_id TEXT NOT NULL,
            chunk_text TEXT NOT NULL,
            chunk_order INTEGER NOT NULL DEFAULT 0,
            chunk_hash TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (library_id) REFERENCES memory_libraries(id) ON DELETE CASCADE
        )
    "#;
    if let Err(e) = conn.execute(memory_library_chunks_table_sql, []) {
        println!("Memory library chunks table migration warning: {}", e);
    }

    let session_memory_reference_history_table_sql = r#"
        CREATE TABLE IF NOT EXISTS session_memory_reference_history (
            session_id TEXT NOT NULL,
            source_type TEXT NOT NULL,
            source_id TEXT NOT NULL,
            message_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (session_id, source_type, source_id),
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
        )
    "#;
    if let Err(e) = conn.execute(session_memory_reference_history_table_sql, []) {
        println!("Session memory reference history table migration warning: {}", e);
    }

    let raw_memory_records_fts_sql = r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS raw_memory_records_fts USING fts5(
            content,
            tokenize = 'trigram',
            content = 'raw_memory_records',
            content_rowid = 'rowid'
        )
    "#;
    if let Err(e) = conn.execute(raw_memory_records_fts_sql, []) {
        println!("Raw memory FTS table migration warning: {}", e);
    }

    let memory_library_chunks_fts_sql = r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS memory_library_chunks_fts USING fts5(
            chunk_text,
            tokenize = 'trigram',
            content = 'memory_library_chunks',
            content_rowid = 'rowid'
        )
    "#;
    if let Err(e) = conn.execute(memory_library_chunks_fts_sql, []) {
        println!("Memory library chunks FTS table migration warning: {}", e);
    }

    let memory_search_triggers = [
        r#"
        CREATE TRIGGER IF NOT EXISTS raw_memory_records_ai
        AFTER INSERT ON raw_memory_records
        BEGIN
            INSERT INTO raw_memory_records_fts(rowid, content)
            VALUES (new.rowid, new.content);
        END;
        "#,
        r#"
        CREATE TRIGGER IF NOT EXISTS raw_memory_records_ad
        AFTER DELETE ON raw_memory_records
        BEGIN
            INSERT INTO raw_memory_records_fts(raw_memory_records_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
        END;
        "#,
        r#"
        CREATE TRIGGER IF NOT EXISTS raw_memory_records_au
        AFTER UPDATE ON raw_memory_records
        BEGIN
            INSERT INTO raw_memory_records_fts(raw_memory_records_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
            INSERT INTO raw_memory_records_fts(rowid, content)
            VALUES (new.rowid, new.content);
        END;
        "#,
        r#"
        CREATE TRIGGER IF NOT EXISTS memory_library_chunks_ai
        AFTER INSERT ON memory_library_chunks
        BEGIN
            INSERT INTO memory_library_chunks_fts(rowid, chunk_text)
            VALUES (new.rowid, new.chunk_text);
        END;
        "#,
        r#"
        CREATE TRIGGER IF NOT EXISTS memory_library_chunks_ad
        AFTER DELETE ON memory_library_chunks
        BEGIN
            INSERT INTO memory_library_chunks_fts(memory_library_chunks_fts, rowid, chunk_text)
            VALUES ('delete', old.rowid, old.chunk_text);
        END;
        "#,
        r#"
        CREATE TRIGGER IF NOT EXISTS memory_library_chunks_au
        AFTER UPDATE ON memory_library_chunks
        BEGIN
            INSERT INTO memory_library_chunks_fts(memory_library_chunks_fts, rowid, chunk_text)
            VALUES ('delete', old.rowid, old.chunk_text);
            INSERT INTO memory_library_chunks_fts(rowid, chunk_text)
            VALUES (new.rowid, new.chunk_text);
        END;
        "#,
    ];
    for migration in memory_search_triggers {
        if let Err(e) = conn.execute(migration, []) {
            println!("Memory search trigger migration warning: {}", e);
        }
    }

    let memory_indexes = [
        "CREATE INDEX IF NOT EXISTS idx_memory_libraries_updated ON memory_libraries(updated_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_raw_memory_records_created ON raw_memory_records(created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_raw_memory_records_project ON raw_memory_records(project_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_raw_memory_records_session ON raw_memory_records(session_id, created_at DESC)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_raw_memory_records_message ON raw_memory_records(message_id) WHERE message_id IS NOT NULL",
        "CREATE INDEX IF NOT EXISTS idx_memory_merge_runs_library_created ON memory_merge_runs(library_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_project_memory_libraries_project ON project_memory_libraries(project_id, created_at ASC)",
        "CREATE INDEX IF NOT EXISTS idx_project_memory_libraries_library ON project_memory_libraries(library_id, created_at ASC)",
        "CREATE INDEX IF NOT EXISTS idx_memory_library_chunks_library_order ON memory_library_chunks(library_id, chunk_order ASC)",
        "CREATE INDEX IF NOT EXISTS idx_memory_library_chunks_hash ON memory_library_chunks(chunk_hash)",
        "CREATE INDEX IF NOT EXISTS idx_session_memory_reference_history_session ON session_memory_reference_history(session_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_session_memory_reference_history_message ON session_memory_reference_history(message_id, created_at DESC)",
    ];
    for migration in memory_indexes {
        if let Err(e) = conn.execute(migration, []) {
            println!("Memory index migration warning: {}", e);
        }
    }

    if let Err(e) = maybe_rebuild_fts_index(&conn, "raw_memory_records_fts", "raw_memory_records") {
        println!("Raw memory FTS rebuild warning: {}", e);
    }
    if let Err(e) = maybe_rebuild_fts_index(&conn, "memory_library_chunks_fts", "memory_library_chunks") {
        println!("Memory chunk FTS rebuild warning: {}", e);
    }

    println!("Database initialized successfully");
    Ok(())
}

fn maybe_rebuild_fts_index(conn: &Connection, fts_table: &str, source_table: &str) -> Result<()> {
    let source_count_sql = format!("SELECT COUNT(*) FROM {}", source_table);
    let fts_count_sql = format!("SELECT COUNT(*) FROM {}", fts_table);
    let source_count: i64 = conn.query_row(&source_count_sql, [], |row| row.get(0))?;
    let fts_count: i64 = conn.query_row(&fts_count_sql, [], |row| row.get(0))?;

    if source_count == fts_count {
        return Ok(());
    }

    let rebuild_sql = format!("INSERT INTO {}({}) VALUES('rebuild')", fts_table, fts_table);
    conn.execute(&rebuild_sql, [])?;
    Ok(())
}
