use anyhow::Result;
use rusqlite::Connection;

/// 鏁版嵁搴撳垵濮嬪寲 SQL 鑴氭湰
const INIT_SQL: &str = r#"
    -- 椤圭洰琛?
    CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        path TEXT NOT NULL UNIQUE,
        description TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_projects_path ON projects(path);

    -- 浼氳瘽琛?
    CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        name TEXT NOT NULL,
        agent_type TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'idle',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_id);

    -- 娑堟伅琛?
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

    -- 鏅鸿兘浣撻厤缃〃
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

    -- MCP 鏈嶅姟鍣ㄩ厤缃〃
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

    -- Skills 閰嶇疆琛紙浠庡競鍦哄畨瑁呯殑 Skills锛?
    CREATE TABLE IF NOT EXISTS skills (
        id TEXT PRIMARY KEY,
        skill_id TEXT,                      -- 甯傚満 Skill ID
        name TEXT NOT NULL,
        description TEXT,
        file_name TEXT NOT NULL,            -- 鏂囦欢鍚?
        path TEXT NOT NULL,                 -- 瀹屾暣璺緞
        source_market TEXT,                 -- 鏉ユ簮甯傚満鍚嶇О
        cli_type TEXT NOT NULL,             -- 鐩爣 CLI (claude, cursor, aider, windsurf)
        scope TEXT NOT NULL DEFAULT 'global', -- 瀹夎鑼冨洿 (global, project)
        project_path TEXT,                  -- 椤圭洰璺緞锛堝鏋滄槸 project scope锛?
        disabled INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_skills_path ON skills(path);
    CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);

    -- 浼氳瘽 MCP 鍏宠仈琛?
    CREATE TABLE IF NOT EXISTS session_mcp (
        session_id TEXT NOT NULL,
        mcp_server_id TEXT NOT NULL,
        enabled INTEGER NOT NULL DEFAULT 1,
        PRIMARY KEY (session_id, mcp_server_id),
        FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
        FOREIGN KEY (mcp_server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE
    );

    -- 涓婚閰嶇疆琛?
    CREATE TABLE IF NOT EXISTS themes (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        colors_light TEXT NOT NULL,
        colors_dark TEXT NOT NULL,
        created_at TEXT NOT NULL
    );

    -- 搴旂敤璁剧疆琛?
    CREATE TABLE IF NOT EXISTS app_settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    -- CLI 璺緞閰嶇疆琛紙鎵嬪姩閰嶇疆锛?
    CREATE TABLE IF NOT EXISTS cli_paths (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        path TEXT NOT NULL,
        version TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_cli_paths_name ON cli_paths(name);

    -- 甯傚満婧愰厤缃〃
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

    -- 宸插畨瑁?MCP 娴嬭瘯缁撴灉琛紙瀛樺偍 CLI 閰嶇疆鏂囦欢涓殑 MCP 娴嬭瘯缁撴灉锛?
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

    -- MCP 瀹夎鍘嗗彶琛?
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

    -- SDK 鏅鸿兘浣?MCP 閰嶇疆琛?
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

    -- SDK 鏅鸿兘浣?Skills 閰嶇疆琛?
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

    -- SDK 鏅鸿兘浣?Plugins 閰嶇疆琛?
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

    -- Provider 閰嶇疆琛?(CC-Switch)
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

    -- 璁″垝琛?(Plan Mode)
    CREATE TABLE IF NOT EXISTS plans (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
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

    -- 浠诲姟琛?(Plan Mode)
    CREATE TABLE IF NOT EXISTS tasks (
        id TEXT PRIMARY KEY,
        plan_id TEXT NOT NULL,
        parent_id TEXT,
        title TEXT NOT NULL,
        description TEXT,
        status TEXT NOT NULL DEFAULT 'pending',
        priority TEXT NOT NULL DEFAULT 'medium',
        assignee TEXT,
        session_id TEXT,
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

    -- 鏅鸿兘浣撴ā鍨嬮厤缃〃
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

    -- 搴旂敤鐘舵€佽〃锛堢獥鍙ｇ姸鎬佹仮澶嶏級
    CREATE TABLE IF NOT EXISTS app_state (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL,
        updated_at INTEGER DEFAULT (strftime('%s', 'now'))
    );

    -- 椤圭洰璁块棶璁板綍琛紙鏈€杩戦」鐩垪琛級
    CREATE TABLE IF NOT EXISTS project_access_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        project_id TEXT NOT NULL UNIQUE,
        last_accessed_at INTEGER NOT NULL,
        access_count INTEGER DEFAULT 1,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_project_access_log_time ON project_access_log(last_accessed_at DESC);

    -- 绐楀彛浼氳瘽閿佸畾琛紙闃叉鍚屼細璇濆绐楀彛锛?
    CREATE TABLE IF NOT EXISTS window_session_locks (
        session_id TEXT PRIMARY KEY,
        window_label TEXT NOT NULL,
        locked_at INTEGER DEFAULT (strftime('%s', 'now'))
    );

    -- 浠诲姟鎷嗗垎浼氳瘽琛紙瀛樺偍AI鍘熷杈撳嚭鍜岃В鏋愮姸鎬侊級
    CREATE TABLE IF NOT EXISTS task_split_sessions (
        id TEXT PRIMARY KEY,
        plan_id TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'processing',
        raw_content TEXT,
        parsed_output TEXT,
        parse_error TEXT,
        granularity INTEGER DEFAULT 20,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_task_split_sessions_plan ON task_split_sessions(plan_id);

    -- 浠诲姟鎵ц鏃ュ織琛?
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

    -- 閮ㄩ棬琛?
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

    -- 浜哄憳琛?
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

    -- 浠诲姟鎵ц缁撴灉蹇収鍘嗗彶琛?
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

    -- 璁板繂鍒嗙被琛紙鐢ㄤ簬 Skills 寮忓眰绾у睍绀猴級
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

    -- 鐢ㄦ埛璁板繂琛?
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

    -- 璁板繂鍘嬬缉鍘嗗彶琛?
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

/// 鍒濆鍖栨暟鎹簱
pub fn init_database() -> Result<()> {
    // 鑾峰彇鎸佷箙鍖栫洰褰?
    let persistence_dir = crate::commands::get_persistence_dir_path()?;
    let db_path = persistence_dir.join("data").join("easy-agent.db");

    // 纭繚鐩綍瀛樺湪
    std::fs::create_dir_all(db_path.parent().unwrap())?;

    println!("Database path: {:?}", db_path);

    // 鎵撳紑鏁版嵁搴撹繛鎺?
    let conn = Connection::open(&db_path)?;

    // 鍚敤澶栭敭绾︽潫锛圫QLite 榛樿涓嶅惎鐢級
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // 鎵ц鍒濆鍖?SQL
    conn.execute_batch(INIT_SQL)?;

    // 鎵ц杩佺Щ锛堝拷鐣ュ垪宸插瓨鍦ㄧ殑閿欒锛?
    // SQLite 涓嶆敮鎸?IF NOT EXISTS 鐢ㄤ簬 ALTER TABLE ADD COLUMN
    // 鎵€浠ユ垜浠渶瑕佸崟鐙墽琛屾瘡鏉¤鍙ュ苟蹇界暐閿欒
    let migrations = [
        "ALTER TABLE mcp_servers ADD COLUMN test_status TEXT",
        "ALTER TABLE mcp_servers ADD COLUMN test_message TEXT",
        "ALTER TABLE mcp_servers ADD COLUMN tool_count INTEGER",
        "ALTER TABLE mcp_servers ADD COLUMN tested_at TEXT",
        "ALTER TABLE mcp_servers ADD COLUMN server_type TEXT DEFAULT 'stdio'",
        "ALTER TABLE mcp_servers ADD COLUMN url TEXT",
        "ALTER TABLE mcp_servers ADD COLUMN headers TEXT",
        // sessions 琛ㄦ坊鍔?pinned 鍜?last_message 瀛楁
        "ALTER TABLE sessions ADD COLUMN pinned INTEGER DEFAULT 0",
        "ALTER TABLE sessions ADD COLUMN last_message TEXT",
        "ALTER TABLE sessions ADD COLUMN error_message TEXT",
    ];

    for migration in migrations {
        // 蹇界暐"鍒楀凡瀛樺湪"閿欒
        if let Err(e) = conn.execute(migration, []) {
            let err_str = e.to_string();
            if !err_str.contains("duplicate column name") {
                println!("Migration warning: {}", e);
            }
        }
    }

    // agents 琛ㄦ坊鍔犳祴璇曠浉鍏冲瓧娈?
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

    // agents 琛ㄦ坊鍔犵粺涓€鏅鸿兘浣撴ā鍨嬪瓧娈?
    // provider: 鎻愪緵鍟?(claude/codex)
    // model_id: 妯″瀷ID
    // custom_model_enabled: 鏄惁鍚敤鑷畾涔夋ā鍨?
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

    // skills 琛ㄦ坊鍔犳柊瀛楁锛堜粠甯傚満瀹夎鐨?skills锛?
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

    // 鍒涘缓 skills 琛ㄧ殑绱㈠紩锛堝鏋滀笉瀛樺湪锛?
    let index_migrations = [
        "CREATE INDEX IF NOT EXISTS idx_skills_path ON skills(path)",
        "CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name)",
    ];

    for migration in index_migrations {
        if let Err(e) = conn.execute(migration, []) {
            println!("Skills index migration warning: {}", e);
        }
    }

    // mcp_install_history 琛ㄨ縼绉伙紙濡傛灉琛ㄤ笉瀛樺湪鍒欏垱寤猴級
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

    // 鍒涘缓绱㈠紩
    let history_index_migrations = [
        "CREATE INDEX IF NOT EXISTS idx_mcp_install_history_created ON mcp_install_history(created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_mcp_install_history_mcp ON mcp_install_history(mcp_name)",
    ];
    for migration in history_index_migrations {
        if let Err(e) = conn.execute(migration, []) {
            println!("MCP install history index migration warning: {}", e);
        }
    }

    // messages 琛ㄦ坊鍔?error_message 瀛楁锛堢敤浜庡瓨鍌ㄥ彂閫佸け璐ョ殑鍘熷洜锛?
    let message_migrations = [
        "ALTER TABLE messages ADD COLUMN attachments TEXT",
        "ALTER TABLE messages ADD COLUMN error_message TEXT",
        "ALTER TABLE messages ADD COLUMN tool_calls TEXT", // JSON string for tool calls
        "ALTER TABLE messages ADD COLUMN thinking TEXT",   // 鎬濊€冨唴瀹癸紙鎵╁睍鎬濈淮妯″瀷锛?
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

    // agent_models 琛ㄨ縼绉伙紙鏅鸿兘浣撴ā鍨嬮厤缃〃锛?
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

    // 鍒涘缓绱㈠紩
    let agent_models_index_sql =
        "CREATE INDEX IF NOT EXISTS idx_agent_models_agent ON agent_models(agent_id)";
    if let Err(e) = conn.execute(agent_models_index_sql, []) {
        println!("Agent models index migration warning: {}", e);
    }

    // agent_models 琛ㄦ坊鍔?context_window 瀛楁
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

    // plans 琛ㄦ坊鍔犳柊瀛楁锛堜换鍔℃媶鍒嗛绮掑害銆佹渶澶ч噸璇曟鏁般€佹墽琛岀姸鎬併€佸綋鍓嶄换鍔D锛?
    let plans_migrations = [
        "ALTER TABLE plans ADD COLUMN granularity INTEGER DEFAULT 20",
        "ALTER TABLE plans ADD COLUMN max_retry_count INTEGER DEFAULT 3",
        "ALTER TABLE plans ADD COLUMN execution_status TEXT DEFAULT 'idle'",
        "ALTER TABLE plans ADD COLUMN current_task_id TEXT",
        "ALTER TABLE plans ADD COLUMN split_agent_id TEXT",
        "ALTER TABLE plans ADD COLUMN split_model_id TEXT",
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

    // tasks 琛ㄦ坊鍔犳柊瀛楁锛堥噸璇曡鏁般€佹渶澶ч噸璇曘€侀敊璇俊鎭€佸疄鐜版楠ゃ€佹祴璇曟楠ゃ€侀獙鏀舵爣鍑嗭級
    let tasks_migrations = [
        "ALTER TABLE tasks ADD COLUMN agent_id TEXT",
        "ALTER TABLE tasks ADD COLUMN model_id TEXT",
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

    // task_split_sessions 琛紙瀛樺偍AI鍘熷杈撳嚭鍜岃В鏋愮姸鎬侊級
    let task_split_sessions_table_sql = r#"
        CREATE TABLE IF NOT EXISTS task_split_sessions (
            id TEXT PRIMARY KEY,
            plan_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'processing',
            raw_content TEXT,
            parsed_output TEXT,
            parse_error TEXT,
            granularity INTEGER DEFAULT 20,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE
        )
    "#;
    if let Err(e) = conn.execute(task_split_sessions_table_sql, []) {
        println!("Task split sessions table migration warning: {}", e);
    }

    // 鍒涘缓绱㈠紩
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

    // task_execution_results 琛紙瀛樺偍浠诲姟鎵ц瀹屾垚/澶辫触鍚庣殑缁撴瀯鍖栫粨鏋滐級
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

    let memory_indexes = [
        "CREATE INDEX IF NOT EXISTS idx_memory_libraries_updated ON memory_libraries(updated_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_raw_memory_records_created ON raw_memory_records(created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_raw_memory_records_project ON raw_memory_records(project_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_raw_memory_records_session ON raw_memory_records(session_id, created_at DESC)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_raw_memory_records_message ON raw_memory_records(message_id) WHERE message_id IS NOT NULL",
        "CREATE INDEX IF NOT EXISTS idx_memory_merge_runs_library_created ON memory_merge_runs(library_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_project_memory_libraries_project ON project_memory_libraries(project_id, created_at ASC)",
        "CREATE INDEX IF NOT EXISTS idx_project_memory_libraries_library ON project_memory_libraries(library_id, created_at ASC)",
    ];
    for migration in memory_indexes {
        if let Err(e) = conn.execute(migration, []) {
            println!("Memory index migration warning: {}", e);
        }
    }

    println!("Database initialized successfully");
    Ok(())
}
