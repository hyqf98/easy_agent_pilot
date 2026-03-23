mod commands;
mod database;
mod logging;
mod scheduler;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(error) = commands::init_persistence_dirs() {
        eprintln!("Failed to initialize persistence directories: {}", error);
    }
    if let Err(error) = logging::init_runtime_logging() {
        eprintln!("Failed to initialize runtime logging: {}", error);
    }

    // 初始化日志，只显示 error 级别的日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .init();

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build());

    // MCP Bridge 插件仅在调试模式下启用
    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(
            tauri_plugin_mcp_bridge::Builder::new()
                .bind_address("127.0.0.1")
                .base_port(9423)
                .build(),
        );
    }

    builder
        .setup(|app| {
            // 初始化持久化目录
            if let Err(e) = commands::init_persistence_dirs() {
                eprintln!("Failed to initialize persistence directories: {}", e);
                crate::logging::write_log(
                    "ERROR",
                    "bootstrap",
                    &format!("Failed to initialize persistence directories: {}", e),
                );
            }

            // 初始化数据库
            if let Err(e) = database::init_database() {
                eprintln!("Failed to initialize database: {}", e);
                crate::logging::write_log(
                    "ERROR",
                    "bootstrap",
                    &format!("Failed to initialize database: {}", e),
                );
            }

            // 初始化策略注册表
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(commands::conversation::init_registry());

            // 恢复待执行的定时计划
            let app_handle = app.handle().clone();
            rt.block_on(async {
                scheduler::restore_scheduled_plans(&app_handle).await;

                // 启动后台调度器（需要在 Tokio 运行时上下文中）
                scheduler::start_scheduler(app_handle);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_persistence_dir,
            commands::check_database_exists,
            commands::cli::detect_cli_tools,
            commands::cli::verify_cli_path,
            commands::cli::list_cli_paths,
            commands::cli::add_cli_path,
            commands::cli::update_cli_path,
            commands::cli::delete_cli_path,
            commands::cli::check_cli_paths_migration_needed,
            commands::cli::get_pending_migration_count,
            commands::cli::migrate_cli_paths_to_agents,
            // CLI Installer commands
            commands::cli_installer::detect_package_managers,
            commands::cli_installer::get_cli_install_options,
            commands::cli_installer::install_cli,
            commands::cli_installer::check_cli_update,
            commands::cli_installer::upgrade_cli,
            commands::cli_installer::cancel_install,
            commands::mcp::list_mcp_servers,
            commands::mcp::add_mcp_server,
            commands::mcp::update_mcp_server,
            commands::mcp::delete_mcp_server,
            commands::mcp::toggle_mcp_server,
            commands::mcp::test_mcp_connection,
            commands::mcp::list_mcp_tools,
            commands::mcp::call_mcp_tool,
            commands::mcp::list_mcp_tools_by_config,
            commands::mcp::call_mcp_tool_by_config,
            commands::marketplace::list_market_sources,
            commands::marketplace::add_market_source,
            commands::marketplace::update_market_source,
            commands::marketplace::delete_market_source,
            commands::marketplace::toggle_market_source,
            commands::marketplace::test_market_source_connection,
            commands::marketplace::test_and_update_market_source,
            commands::mcpmarket_source::list_marketplace_source_options,
            commands::mcp_market::fetch_mcp_market,
            commands::mcp_market::fetch_mcp_market_detail,
            commands::mcp_market::install_mcp_to_cli,
            commands::mcp_market::rollback_mcp_install,
            commands::mcp_market::list_installed_mcps,
            commands::mcp_market::toggle_installed_mcp,
            commands::mcp_market::uninstall_mcp,
            commands::mcp_market::check_mcp_updates,
            commands::mcp_market::update_mcp,
            commands::mcp_market::update_installed_mcp,
            commands::mcp_market::test_installed_mcp_connection,
            commands::mcp_market::get_installed_mcp_test_result,
            commands::mcp_market::save_install_history,
            commands::mcp_market::get_install_history,
            commands::mcp_market::manual_rollback_install,
            commands::skills_market::fetch_skills_market,
            commands::skills_market::fetch_skill_market_detail,
            commands::skills_market::list_installed_skills,
            commands::skills_market::install_skill_to_cli,
            commands::skills_market::toggle_installed_skill,
            commands::skills_market::uninstall_skill,
            commands::skills_market::check_skill_updates,
            commands::skills_market::update_skill,
            commands::plugins_market::fetch_plugins_market,
            commands::plugins_market::fetch_plugin_detail,
            commands::plugins_market::install_plugin,
            commands::plugins_market::list_installed_plugins,
            commands::plugins_market::toggle_plugin,
            commands::plugins_market::uninstall_plugin,
            commands::install::create_install_session,
            commands::install::record_create_file,
            commands::install::record_create_dir,
            commands::install::record_modify_file,
            commands::install::record_delete_file,
            commands::install::rollback_install,
            commands::install::complete_install,
            commands::install::get_install_session_status,
            commands::install::cancel_install_session,
            commands::install::list_pending_install_sessions,
            commands::install::list_all_install_sessions,
            commands::install::cleanup_install_session,
            commands::project::list_projects,
            commands::project::create_project,
            commands::project::update_project,
            commands::project::delete_project,
            commands::project::clear_project_runtime_data,
            commands::project::validate_project_path,
            commands::project::list_project_files,
            commands::project::load_directory_children,
            commands::project::list_all_project_files_flat,
            commands::project::search_file_mentions,
            commands::project::rename_file,
            commands::project::delete_file,
            commands::project::batch_delete_files,
            commands::project::move_file,
            commands::file_editor::read_project_file,
            commands::file_editor::write_project_file,
            commands::file_editor::detect_file_language,
            commands::lsp::get_lsp_storage_dir,
            commands::lsp::list_lsp_servers,
            commands::lsp::download_lsp_server,
            commands::lsp::remove_lsp_server,
            commands::lsp::activate_lsp_for_file,
            commands::session::list_sessions,
            commands::session::create_session,
            commands::session::update_session,
            commands::session::delete_session,
            commands::session::toggle_session_pin,
            commands::message::list_messages,
            commands::message::create_message,
            commands::message::update_message,
            commands::message::update_message_fields,
            commands::message::delete_message,
            commands::message::clear_session_messages,
            commands::message::upload_session_images,
            commands::message::delete_uploaded_image,
            commands::mini_panel::ensure_mini_panel_state,
            commands::mini_panel::set_mini_panel_working_directory,
            commands::mini_panel::get_mini_panel_default_shortcut,
            commands::mini_panel::suggest_mini_panel_directories,
            commands::mini_panel::register_mini_panel_windows_shortcut,
            commands::mini_panel::unregister_mini_panel_windows_shortcut,
            commands::mini_panel::capture_mini_panel_native_shortcut_once,
            commands::mini_panel::show_mini_panel,
            commands::mini_panel::hide_mini_panel,
            commands::mini_panel::toggle_mini_panel,
            commands::agent::list_agents,
            commands::agent::create_agent,
            commands::agent::update_agent,
            commands::agent::delete_agent,
            commands::agent::test_agent_connection,
            // Agent MCP Config commands
            commands::agent_config::list_agent_mcp_configs,
            commands::agent_config::create_agent_mcp_config,
            commands::agent_config::update_agent_mcp_config,
            commands::agent_config::delete_agent_mcp_config,
            // Agent Skills Config commands
            commands::agent_config::list_agent_skills_configs,
            commands::agent_config::create_agent_skills_config,
            commands::agent_config::update_agent_skills_config,
            commands::agent_config::delete_agent_skills_config,
            // Agent Plugins Config commands
            commands::agent_config::list_agent_plugins_configs,
            commands::agent_config::create_agent_plugins_config,
            commands::agent_config::update_agent_plugins_config,
            commands::agent_config::delete_agent_plugins_config,
            // Agent Models Config commands
            commands::agent_config::list_agent_models,
            commands::agent_config::create_agent_model,
            commands::agent_config::create_builtin_models,
            commands::agent_config::update_agent_model,
            commands::agent_config::delete_agent_model,
            commands::data::export_all_data,
            commands::data::export_data_to_file,
            commands::data::export_selected_data,
            commands::data::export_selected_to_file,
            commands::data::get_data_management_stats,
            commands::data::validate_import_data,
            commands::data::import_data_from_file,
            commands::data::clear_all_data,
            commands::settings::get_app_setting,
            commands::settings::get_all_app_settings,
            commands::settings::save_app_setting,
            commands::settings::save_app_settings,
            commands::settings::delete_app_setting,
            commands::settings::clear_app_settings,
            commands::runtime_log::get_runtime_log_summary_command,
            commands::runtime_log::list_runtime_log_files_command,
            commands::runtime_log::read_runtime_log_file_command,
            commands::runtime_log::clear_runtime_log_files_command,
            commands::scan::scan_cli_config,
            commands::scan::scan_claude_mcp_list,
            commands::scan::scan_cli_sessions,
            commands::scan::list_agent_cli_session_projects,
            commands::scan::list_agent_cli_sessions,
            commands::scan::read_cli_session_detail,
            commands::scan::delete_cli_session,
            commands::scan::delete_cli_sessions,
            // CLI Config commands
            commands::cli_config::get_cli_config_paths,
            commands::cli_config::read_cli_config,
            commands::cli_config::write_cli_config,
            commands::cli_config::update_cli_mcp_config,
            commands::cli_config::delete_cli_mcp_config,
            commands::cli_config::sync_cli_items,
            commands::cli_config::open_config_file,
            commands::cli_config::get_cli_capabilities,
            // Provider Profile commands (CC-Switch)
            commands::provider_profile::list_provider_profiles,
            commands::provider_profile::get_provider_profile,
            commands::provider_profile::create_provider_profile,
            commands::provider_profile::update_provider_profile,
            commands::provider_profile::delete_provider_profile,
            commands::provider_profile::get_active_provider_profile,
            commands::provider_profile::switch_provider_profile,
            commands::provider_profile::read_current_cli_config,
            commands::provider_profile::read_cli_connection_info,
            commands::provider_profile::read_all_cli_connections,
            // Skill Plugin commands
            commands::skill_plugin::read_skill_file,
            commands::skill_plugin::list_skill_references,
            commands::skill_plugin::read_reference_file,
            commands::skill_plugin::create_cli_skill_scaffold,
            commands::skill_plugin::get_plugin_details,
            commands::skill_plugin::delete_skill_directory,
            commands::skill_plugin::delete_plugin_directory,
            commands::skill_plugin::read_file_content,
            commands::skill_plugin::write_file_content,
            commands::skill_plugin::list_directory_files,
            // Conversation commands
            commands::conversation::execute_claude_cli,
            commands::conversation::execute_codex_cli,
            commands::conversation::execute_claude_sdk,
            commands::conversation::execute_codex_sdk,
            commands::conversation::executor::execute_agent,
            commands::conversation::abort_cli_execution,
            commands::conversation::abort_sdk_execution,
            // Plan Mode commands
            commands::plan::list_plans,
            commands::plan::get_plan,
            commands::plan::create_plan,
            commands::plan::update_plan,
            commands::plan::delete_plan,
            commands::plan::list_scheduled_plans,
            commands::plan::cancel_plan_schedule,
            commands::plan_split::get_plan_split_session,
            commands::plan_split::list_plan_split_logs,
            commands::plan_split::start_plan_split,
            commands::plan_split::resume_plan_split,
            commands::plan_split::submit_plan_split_form,
            commands::plan_split::stop_plan_split,
            commands::plan_split::clear_plan_split_session,
            // Task commands
            commands::task::list_tasks,
            commands::task::get_task,
            commands::task::get_task_by_session_id,
            commands::task::create_task,
            commands::task::update_task,
            commands::task::delete_task,
            commands::task::reorder_tasks,
            commands::task::list_subtasks,
            commands::task::retry_task,
            commands::task::batch_update_status,
            commands::task::stop_task,
            commands::task::batch_create_tasks,
            commands::task::save_split_session,
            commands::task::get_split_session,
            commands::task::delete_split_session,
            // Task Execution commands
            commands::task_execution::create_task_execution_log,
            commands::task_execution::list_task_execution_logs,
            commands::task_execution::clear_task_execution_logs,
            commands::task_execution::get_task_execution_log_stats,
            commands::task_execution::save_task_execution_result,
            commands::task_execution::list_recent_plan_results,
            commands::task_execution::list_plan_execution_progress,
            commands::task_execution::clear_plan_execution_results,
            // Memory commands
            commands::memory::list_memory_libraries,
            commands::memory::get_memory_library,
            commands::memory::create_memory_library,
            commands::memory::update_memory_library,
            commands::memory::delete_memory_library,
            commands::memory::list_raw_memory_records,
            commands::memory::create_raw_memory_record,
            commands::memory::update_raw_memory_record,
            commands::memory::delete_raw_memory_record,
            commands::memory::batch_delete_raw_memory_records,
            commands::memory::capture_user_message,
            commands::memory::list_memory_merge_runs,
            commands::memory::merge_raw_memories_into_library,
            // App State commands
            commands::app_state::get_app_state,
            commands::app_state::set_app_state,
            commands::app_state::get_app_states,
            // Project Access commands
            commands::project_access::record_project_access,
            commands::project_access::get_recent_projects,
            commands::project_access::delete_project_access_log,
            // Window commands
            commands::window::open_project_in_new_window,
            commands::window::get_window_context,
            commands::window::lock_session,
            commands::window::release_session,
            commands::window::is_session_locked,
            commands::window::release_window_sessions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
