mod agent;
mod backup;
mod commands;
mod db;
mod mcp;
mod recurrence;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tack", version, about = "CLI for tack task manager")]
struct Cli {
    #[arg(long, global = true, help = "Override database path")]
    db: Option<PathBuf>,

    #[arg(long, global = true, help = "Output machine-readable JSON")]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage projects
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// Manage tasks
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    /// Manage subtasks
    Subtask {
        #[command(subcommand)]
        action: SubtaskAction,
    },
    /// Manage labels
    Label {
        #[command(subcommand)]
        action: LabelAction,
    },
    /// Manage attachments
    Attachment {
        #[command(subcommand)]
        action: AttachmentAction,
    },
    /// View activity log
    Activity {
        #[command(subcommand)]
        action: ActivityAction,
    },
    /// Data export/import/reset
    Data {
        #[command(subcommand)]
        action: DataAction,
    },
    /// Manage settings
    Settings {
        #[command(subcommand)]
        action: SettingsAction,
    },
    /// Manage the live server
    Live {
        #[command(subcommand)]
        action: LiveAction,
    },
    /// Show workspace stats
    Stats,
    /// List tasks due today
    Today,
    /// Control the running app's ui (ai agent access)
    Ui {
        #[command(subcommand)]
        action: UiAction,
    },
    /// Run an mcp stdio server exposing the app's ui tools
    Mcp,
}

#[derive(Subcommand)]
enum UiAction {
    /// Check whether the agent api is reachable
    Status,
    /// List interactable elements with ref ids (--text for compact lines)
    Snapshot {
        #[arg(long, help = "Print compact text lines instead of json")]
        text: bool,
    },
    /// Capture the app window as png
    Screenshot {
        #[arg(long, help = "Output file path")]
        out: Option<String>,
    },
    /// Click an element: --ref N or --x X --y Y
    Click {
        #[arg(long, alias = "ref")]
        reference: Option<u64>,
        #[arg(long)]
        x: Option<f64>,
        #[arg(long)]
        y: Option<f64>,
        #[arg(long)]
        double: bool,
        #[arg(long)]
        right: bool,
    },
    /// Double-click an element: --ref N or --x X --y Y
    Dblclick {
        #[arg(long, alias = "ref")]
        reference: Option<u64>,
        #[arg(long)]
        x: Option<f64>,
        #[arg(long)]
        y: Option<f64>,
    },
    /// Right-click an element: --ref N or --x X --y Y
    Rightclick {
        #[arg(long, alias = "ref")]
        reference: Option<u64>,
        #[arg(long)]
        x: Option<f64>,
        #[arg(long)]
        y: Option<f64>,
    },
    /// Hover an element: --ref N or --x X --y Y
    Hover {
        #[arg(long, alias = "ref")]
        reference: Option<u64>,
        #[arg(long)]
        x: Option<f64>,
        #[arg(long)]
        y: Option<f64>,
    },
    /// Set an input's text: --ref N --text "..."
    Type {
        #[arg(long, alias = "ref")]
        reference: Option<u64>,
        #[arg(long)]
        x: Option<f64>,
        #[arg(long)]
        y: Option<f64>,
        #[arg(long)]
        text: String,
    },
    /// Press a key or combo: enter, escape, meta+k (meta = cmd)
    Key { keys: String },
    /// Scroll the page or a container
    Scroll {
        #[arg(long, alias = "ref")]
        reference: Option<u64>,
        #[arg(long)]
        x: Option<f64>,
        #[arg(long)]
        y: Option<f64>,
        #[arg(long, default_value_t = 0.0)]
        dx: f64,
        #[arg(long, default_value_t = 400.0)]
        dy: f64,
    },
    /// Drag between points or elements
    Drag {
        #[arg(long, help = "Start point \"x,y\"")]
        from: Option<String>,
        #[arg(long, help = "End point \"x,y\"")]
        to: Option<String>,
        #[arg(long)]
        from_ref: Option<u64>,
        #[arg(long)]
        to_ref: Option<u64>,
    },
    /// Navigate to an app route (/, /settings, /stats, /trash)
    Goto { path: String },
    /// Wait for the ui to settle (max 5000 ms)
    Wait {
        #[arg(long, default_value_t = 500)]
        ms: u64,
    },
    /// Run javascript in the app and print the result
    Eval { code: String },
    /// Ask the user to approve an action in a native dialog
    Confirm {
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// Create a new project
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        prefix: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// List all projects
    List,
    /// Update a project
    Update {
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        prefix: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a project and its tasks
    Delete { id: String },
}

#[derive(Subcommand)]
enum TaskAction {
    /// Create a new task
    Create {
        #[arg(long)]
        title: String,
        #[arg(long, help = "Project ID")]
        project: Option<String>,
        #[arg(long, help = "Project prefix (alternative to --project)")]
        project_prefix: Option<String>,
        #[arg(long, help = "Status: todo, in_progress, done, canceled")]
        status: Option<String>,
        #[arg(
            long,
            help = "Priority 0-4 (0=none, 1=urgent, 2=high, 3=medium, 4=low)"
        )]
        priority: Option<i32>,
        #[arg(long, help = "Due date (YYYY-MM-DD)")]
        due_date: Option<String>,
        #[arg(long, help = "End date (YYYY-MM-DD)")]
        end_date: Option<String>,
        #[arg(long, help = "Reminder (ISO 8601, e.g. 2026-09-14T09:00)")]
        reminder: Option<String>,
        #[arg(
            long,
            help = "Repeat schedule: daily, weekly:<mon..sun|1-7>, monthly:<1-31>"
        )]
        repeat: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, help = "Parent task id or number")]
        parent: Option<String>,
        #[arg(long, help = "Estimate: 90m, 2h, 1h30m")]
        estimate: Option<String>,
    },
    /// List tasks with optional filters
    List {
        #[arg(long, help = "Filter by project ID")]
        project: Option<String>,
        #[arg(long, help = "Filter by project prefix")]
        project_prefix: Option<String>,
        #[arg(long, help = "Filter by status")]
        status: Option<String>,
        #[arg(long, help = "Filter by priority 0-4")]
        priority: Option<i32>,
        #[arg(long, help = "Show only pinned tasks")]
        pinned: bool,
        #[arg(long, help = "Show tasks updated since timestamp (ISO 8601)")]
        since: Option<String>,
    },
    /// Full-text search (title, description, subtasks, labels, project, issue number)
    Search {
        #[arg(long, help = "Search query")]
        query: String,
        #[arg(long, help = "Filter by project ID")]
        project: Option<String>,
        #[arg(long, help = "Filter by project prefix")]
        project_prefix: Option<String>,
        #[arg(long, help = "Filter by status")]
        status: Option<String>,
        #[arg(long, help = "Max results (default 50)")]
        limit: Option<i64>,
    },
    /// Show task details
    Show { id: String },
    /// Update a task
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, help = "Status: todo, in_progress, done, canceled")]
        status: Option<String>,
        #[arg(long, help = "Priority 0-4")]
        priority: Option<i32>,
        #[arg(long, help = "Due date (YYYY-MM-DD), use empty string to clear")]
        due_date: Option<String>,
        #[arg(long, help = "End date (YYYY-MM-DD), use empty string to clear")]
        end_date: Option<String>,
        #[arg(long, help = "Reminder (ISO 8601); use empty string to clear")]
        reminder: Option<String>,
        #[arg(
            long,
            help = "Repeat schedule: daily, weekly:<mon..sun|1-7>, monthly:<1-31>; use empty string to clear"
        )]
        repeat: Option<String>,
        #[arg(long, help = "Parent task id or number; empty clears")]
        parent: Option<String>,
        #[arg(long, help = "Estimate: 90m, 2h, 1h30m; empty clears")]
        estimate: Option<String>,
    },
    /// Delete a task
    Delete { id: String },
    /// Duplicate a task
    Duplicate { id: String },
    /// Pin a task
    Pin { id: String },
    /// Unpin a task
    Unpin { id: String },
    /// Move task to a different project
    Move {
        id: String,
        #[arg(long, help = "Target project ID (use empty for no project)")]
        project: Option<String>,
        #[arg(long, help = "Target project prefix")]
        project_prefix: Option<String>,
    },
    /// Delete multiple tasks
    BulkDelete {
        #[arg(long, help = "Comma-separated task IDs")]
        ids: String,
    },
    /// Update status of multiple tasks
    BulkStatus {
        #[arg(long, help = "Comma-separated task IDs")]
        ids: String,
        #[arg(long, help = "New status")]
        status: String,
    },
    /// Update priority of multiple tasks
    BulkPriority {
        #[arg(long, help = "Comma-separated task IDs")]
        ids: String,
        #[arg(long, help = "New priority 0-4")]
        priority: i32,
    },
    /// Move multiple tasks to a project
    BulkMove {
        #[arg(long, help = "Comma-separated task IDs")]
        ids: String,
        #[arg(long, help = "Target project ID")]
        project: Option<String>,
        #[arg(long, help = "Target project prefix")]
        project_prefix: Option<String>,
    },
    /// Add or remove a label on multiple tasks
    BulkLabel {
        #[arg(long, help = "Comma-separated task IDs")]
        ids: String,
        #[arg(long, help = "Label ID (see `tack label list`)")]
        label: String,
        #[arg(long, help = "Remove the label instead of adding it")]
        remove: bool,
    },
    /// List child tasks
    Children { id: String },
    /// Start time tracking on a task
    Start { id: String },
    /// Stop the running timer on a task
    Stop { id: String },
    /// Manage task relations (blocks, relates, duplicates)
    Relation {
        #[command(subcommand)]
        action: RelationAction,
    },
    /// List trashed tasks
    Trash,
    /// Restore a task from trash
    Restore { id: String },
    /// Permanently delete a task from trash
    PermanentDelete { id: String },
    /// Permanently delete all trashed tasks
    EmptyTrash,
}

#[derive(Subcommand)]
enum RelationAction {
    /// Add a relation: blocks, relates, duplicates
    Add {
        id: String,
        other: String,
        #[arg(long, help = "blocks, relates or duplicates")]
        kind: String,
    },
    /// Remove a relation
    Remove {
        id: String,
        other: String,
        #[arg(long, help = "blocks, relates or duplicates")]
        kind: String,
    },
    /// List a task's relations
    List { id: String },
}

#[derive(Subcommand)]
enum SubtaskAction {
    /// Add a subtask to a task
    Add {
        #[arg(long)]
        task: String,
        #[arg(long)]
        title: String,
    },
    /// List subtasks of a task
    List {
        #[arg(long)]
        task: String,
    },
    /// Toggle subtask completion
    Toggle { id: String },
    /// Rename a subtask
    Rename {
        id: String,
        #[arg(long)]
        title: String,
    },
    /// Delete a subtask
    Delete { id: String },
}

#[derive(Subcommand)]
enum LabelAction {
    /// Create a new label
    Create {
        #[arg(long)]
        name: String,
        #[arg(
            long,
            help = "Color: gray, blue, green, amber, red, purple, pink, teal, orange, indigo"
        )]
        color: String,
    },
    /// List all labels
    List,
    /// Update a label
    Update {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, help = "Color")]
        color: Option<String>,
    },
    /// Delete a label
    Delete { id: String },
    /// Assign labels to a task (replaces existing)
    Assign {
        #[arg(long)]
        task: String,
        #[arg(long, help = "Comma-separated label IDs")]
        labels: String,
    },
    /// Show labels assigned to a task
    Show {
        #[arg(long)]
        task: String,
    },
}

#[derive(Subcommand)]
enum AttachmentAction {
    /// Add a file attachment to a task
    Add {
        #[arg(long)]
        task: String,
        #[arg(long)]
        file: String,
    },
    /// List attachments of a task
    List {
        #[arg(long)]
        task: String,
    },
    /// Delete an attachment
    Delete { id: String },
    /// Download an attachment to a file
    Download {
        id: String,
        #[arg(long, help = "Output path (defaults to original filename)")]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
enum ActivityAction {
    /// List recent activity for a task
    List {
        #[arg(long)]
        task: String,
    },
}

#[derive(Subcommand)]
enum DataAction {
    /// Export all data to a file
    Export {
        #[arg(long, help = "Output file path (defaults to tack-export.json)")]
        output: Option<String>,
        #[arg(long, default_value = "json", help = "json, csv or markdown")]
        format: String,
    },
    /// Import data from a JSON file
    Import {
        #[arg(long)]
        file: String,
    },
    /// Create a local backup of the database and attachments
    Backup {
        #[arg(long, help = "How many backups to keep (default 7)")]
        keep: Option<usize>,
    },
    /// Restore database and attachments from a backup by name
    Restore {
        #[arg(help = "Backup name (see: tack data backup-list or backups/ directory)")]
        name: String,
    },
    /// List available backups
    BackupList,
    /// Delete a backup by name
    BackupDelete {
        #[arg(help = "Backup name (see: tack data backup-list)")]
        name: String,
    },
    /// Reset the database (delete all data)
    Reset,
}

#[derive(Subcommand)]
enum SettingsAction {
    /// Show all settings
    Get,
    /// Set a setting value
    Set { key: String, value: String },
}

#[derive(Subcommand)]
enum LiveAction {
    /// Enable the live server
    On {
        #[arg(long, help = "Port to listen on (default 17890)")]
        port: Option<u16>,
    },
    /// Disable the live server
    Off,
    /// Show live server status
    Status,
    /// Watch db changes in real time (server-sent events)
    Watch,
}

fn parse_ids(s: &str) -> Vec<String> {
    s.split(',')
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect()
}

fn main() {
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(db::get_db_path);
    let json = cli.json;

    let conn = match db::connect(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let result = match cli.command {
        Commands::Project { action } => match action {
            ProjectAction::Create {
                name,
                prefix,
                description,
            } => commands::project::create(&conn, json, &name, &prefix, description.as_deref()),
            ProjectAction::List => commands::project::list(&conn, json),
            ProjectAction::Update {
                id,
                name,
                prefix,
                description,
            } => {
                commands::project::update(&conn, json, &id, &name, &prefix, description.as_deref())
            }
            ProjectAction::Delete { id } => commands::project::delete(&conn, json, &id),
        },
        Commands::Task { action } => match action {
            TaskAction::Create {
                title,
                project,
                project_prefix,
                status,
                priority,
                due_date,
                end_date,
                reminder,
                repeat,
                description,
                parent,
                estimate,
            } => {
                let dd = due_date.as_deref().filter(|s| !s.is_empty());
                let ed = end_date.as_deref().filter(|s| !s.is_empty());
                commands::task::create(
                    &conn,
                    json,
                    &title,
                    project.as_deref(),
                    project_prefix.as_deref(),
                    status.as_deref(),
                    priority,
                    dd,
                    ed,
                    description.as_deref(),
                    reminder.as_deref(),
                    repeat.as_deref(),
                    parent.as_deref(),
                    estimate.as_deref(),
                )
            }
            TaskAction::List {
                project,
                project_prefix,
                status,
                priority,
                pinned,
                since,
            } => commands::task::list(
                &conn,
                json,
                project.as_deref(),
                project_prefix.as_deref(),
                status.as_deref(),
                priority,
                pinned,
                since.as_deref(),
            ),
            TaskAction::Search {
                query,
                project,
                project_prefix,
                status,
                limit,
            } => commands::task::search(
                &conn,
                json,
                &query,
                project.as_deref(),
                project_prefix.as_deref(),
                status.as_deref(),
                limit.unwrap_or(50),
            ),
            TaskAction::Show { id } => commands::task::show(&conn, json, &id),
            TaskAction::Update {
                id,
                title,
                description,
                status,
                priority,
                due_date,
                end_date,
                reminder,
                repeat,
                parent,
                estimate,
            } => commands::task::update(
                &conn,
                json,
                &id,
                title.as_deref(),
                description.as_deref(),
                status.as_deref(),
                priority,
                due_date.as_deref(),
                end_date.as_deref(),
                reminder.as_deref(),
                repeat.as_deref(),
                parent.as_deref(),
                estimate.as_deref(),
            ),
            TaskAction::Delete { id } => commands::task::delete(&conn, json, &id),
            TaskAction::Duplicate { id } => commands::task::duplicate(&conn, json, &id),
            TaskAction::Pin { id } => commands::task::toggle_pin(&conn, json, &id, true),
            TaskAction::Unpin { id } => commands::task::toggle_pin(&conn, json, &id, false),
            TaskAction::Move {
                id,
                project,
                project_prefix,
            } => commands::task::move_to_project(
                &conn,
                json,
                &id,
                project.as_deref(),
                project_prefix.as_deref(),
            ),
            TaskAction::BulkDelete { ids } => {
                commands::task::bulk_delete(&conn, json, &parse_ids(&ids))
            }
            TaskAction::BulkStatus { ids, status } => {
                commands::task::bulk_status(&conn, json, &parse_ids(&ids), &status)
            }
            TaskAction::BulkPriority { ids, priority } => {
                commands::task::bulk_priority(&conn, json, &parse_ids(&ids), priority)
            }
            TaskAction::BulkMove {
                ids,
                project,
                project_prefix,
            } => commands::task::bulk_move(
                &conn,
                json,
                &parse_ids(&ids),
                project.as_deref(),
                project_prefix.as_deref(),
            ),
            TaskAction::BulkLabel { ids, label, remove } => {
                commands::task::bulk_label(&conn, json, &parse_ids(&ids), &label, remove)
            }
            TaskAction::Children { id } => commands::task::children(&conn, json, &id),
            TaskAction::Start { id } => commands::task::timer_start(&conn, json, &id),
            TaskAction::Stop { id } => commands::task::timer_stop(&conn, json, &id),
            TaskAction::Relation { action } => match action {
                RelationAction::Add { id, other, kind } => {
                    commands::task::relation_add(&conn, json, &id, &other, &kind)
                }
                RelationAction::Remove { id, other, kind } => {
                    commands::task::relation_remove(&conn, json, &id, &other, &kind)
                }
                RelationAction::List { id } => {
                    commands::task::relation_list(&conn, json, &id)
                }
            },
            TaskAction::Trash => commands::task::trash_list(&conn, json),
            TaskAction::Restore { id } => commands::task::restore(&conn, json, &id),
            TaskAction::PermanentDelete { id } => {
                commands::task::permanent_delete(&conn, json, &id)
            }
            TaskAction::EmptyTrash => commands::task::empty_trash(&conn, json),
        },
        Commands::Subtask { action } => match action {
            SubtaskAction::Add { task, title } => {
                commands::subtask::add(&conn, json, &task, &title)
            }
            SubtaskAction::List { task } => commands::subtask::list(&conn, json, &task),
            SubtaskAction::Toggle { id } => commands::subtask::toggle(&conn, json, &id),
            SubtaskAction::Rename { id, title } => {
                commands::subtask::rename(&conn, json, &id, &title)
            }
            SubtaskAction::Delete { id } => commands::subtask::delete(&conn, json, &id),
        },
        Commands::Label { action } => match action {
            LabelAction::Create { name, color } => {
                commands::label::create(&conn, json, &name, &color)
            }
            LabelAction::List => commands::label::list(&conn, json),
            LabelAction::Update { id, name, color } => {
                commands::label::update(&conn, json, &id, name.as_deref(), color.as_deref())
            }
            LabelAction::Delete { id } => commands::label::delete(&conn, json, &id),
            LabelAction::Assign { task, labels } => {
                commands::label::assign(&conn, json, &task, &parse_ids(&labels))
            }
            LabelAction::Show { task } => commands::label::show(&conn, json, &task),
        },
        Commands::Attachment { action } => match action {
            AttachmentAction::Add { task, file } => {
                commands::attachment::add(&conn, json, &task, &file)
            }
            AttachmentAction::List { task } => commands::attachment::list(&conn, json, &task),
            AttachmentAction::Delete { id } => commands::attachment::delete(&conn, json, &id),
            AttachmentAction::Download { id, output } => {
                commands::attachment::download(&conn, json, &id, output.as_deref().unwrap_or(""))
            }
        },
        Commands::Activity { action } => match action {
            ActivityAction::List { task } => commands::activity::list(&conn, json, &task),
        },
        Commands::Data { action } => match action {
            DataAction::Export { output, format } => {
                commands::data::export(&conn, json, output.as_deref().unwrap_or(""), &format)
            }
            DataAction::Import { file } => commands::data::import(&conn, json, &file),
            DataAction::Backup { keep } => {
                commands::data::backup(json, &db_path, keep.unwrap_or(7))
            }
            DataAction::Restore { name } => commands::data::restore(json, &db_path, &name),
            DataAction::BackupList => commands::data::backup_list(json, &db_path),
            DataAction::BackupDelete { name } => {
                commands::data::backup_delete(json, &db_path, &name)
            }
            DataAction::Reset => commands::data::reset(&conn, json),
        },
        Commands::Settings { action } => match action {
            SettingsAction::Get => commands::settings::get(&conn, json),
            SettingsAction::Set { key, value } => {
                commands::settings::set(&conn, json, &key, &value)
            }
        },
        Commands::Live { action } => match action {
            LiveAction::On { port } => commands::live::on(&conn, json, port),
            LiveAction::Off => commands::live::off(&conn, json),
            LiveAction::Status => commands::live::status(&conn, json),
            LiveAction::Watch => commands::live::watch(&conn, json),
        },
        Commands::Stats => commands::stats::stats(&conn, json),
        Commands::Today => commands::task::today(&conn, json),
        Commands::Ui { action } => match action {
            UiAction::Status => commands::ui::status(&conn, json),
            UiAction::Snapshot { text } => commands::ui::snapshot(&conn, json, text),
            UiAction::Screenshot { out } => commands::ui::screenshot(&conn, out),
            UiAction::Click {
                reference,
                x,
                y,
                double,
                right,
            } => commands::ui::click(&conn, json, reference, x, y, double, right),
            UiAction::Dblclick { reference, x, y } => {
                commands::ui::click(&conn, json, reference, x, y, true, false)
            }
            UiAction::Rightclick { reference, x, y } => {
                commands::ui::click(&conn, json, reference, x, y, false, true)
            }
            UiAction::Hover { reference, x, y } => {
                commands::ui::hover(&conn, json, reference, x, y)
            }
            UiAction::Type {
                reference,
                x,
                y,
                text,
            } => commands::ui::type_text(&conn, json, reference, x, y, text),
            UiAction::Key { keys } => commands::ui::key(&conn, json, keys),
            UiAction::Scroll {
                reference,
                x,
                y,
                dx,
                dy,
            } => commands::ui::scroll(&conn, json, reference, x, y, dx, dy),
            UiAction::Drag {
                from,
                to,
                from_ref,
                to_ref,
            } => commands::ui::drag(&conn, json, from, to, from_ref, to_ref),
            UiAction::Goto { path } => commands::ui::goto(&conn, json, path),
            UiAction::Wait { ms } => commands::ui::wait(&conn, json, ms),
            UiAction::Eval { code } => commands::ui::eval(&conn, json, code),
            UiAction::Confirm { title, body } => commands::ui::confirm(&conn, json, title, body),
        },
        Commands::Mcp => mcp::serve(&conn),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
