mod backup;
mod db;
mod commands;

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
    Delete {
        id: String,
    },
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
        #[arg(long, help = "Priority 0-4 (0=none, 1=urgent, 2=high, 3=medium, 4=low)")]
        priority: Option<i32>,
        #[arg(long, help = "Due date (YYYY-MM-DD)")]
        due_date: Option<String>,
        #[arg(long, help = "End date (YYYY-MM-DD)")]
        end_date: Option<String>,
        #[arg(long)]
        description: Option<String>,
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
    /// Show task details
    Show {
        id: String,
    },
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
    },
    /// Delete a task
    Delete {
        id: String,
    },
    /// Duplicate a task
    Duplicate {
        id: String,
    },
    /// Pin a task
    Pin {
        id: String,
    },
    /// Unpin a task
    Unpin {
        id: String,
    },
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
    /// List trashed tasks
    Trash,
    /// Restore a task from trash
    Restore {
        id: String,
    },
    /// Permanently delete a task from trash
    PermanentDelete {
        id: String,
    },
    /// Permanently delete all trashed tasks
    EmptyTrash,
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
    Toggle {
        id: String,
    },
    /// Rename a subtask
    Rename {
        id: String,
        #[arg(long)]
        title: String,
    },
    /// Delete a subtask
    Delete {
        id: String,
    },
}

#[derive(Subcommand)]
enum LabelAction {
    /// Create a new label
    Create {
        #[arg(long)]
        name: String,
        #[arg(long, help = "Color: gray, blue, green, amber, red, purple, pink, teal, orange, indigo")]
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
    Delete {
        id: String,
    },
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
    Delete {
        id: String,
    },
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
    /// Export all data to a JSON file
    Export {
        #[arg(long, help = "Output file path (defaults to tack-export.json)")]
        output: Option<String>,
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
    Set {
        key: String,
        value: String,
    },
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
}

fn parse_ids(s: &str) -> Vec<String> {
    s.split(',').map(|id| id.trim().to_string()).filter(|id| !id.is_empty()).collect()
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
            ProjectAction::Create { name, prefix, description } => {
                commands::project::create(&conn, json, &name, &prefix, description.as_deref())
            }
            ProjectAction::List => {
                commands::project::list(&conn, json)
            }
            ProjectAction::Update { id, name, prefix, description } => {
                commands::project::update(&conn, json, &id, &name, &prefix, description.as_deref())
            }
            ProjectAction::Delete { id } => {
                commands::project::delete(&conn, json, &id)
            }
        },
        Commands::Task { action } => match action {
            TaskAction::Create { title, project, project_prefix, status, priority, due_date, end_date, description } => {
                let dd = due_date.as_deref().filter(|s| !s.is_empty());
                let ed = end_date.as_deref().filter(|s| !s.is_empty());
                commands::task::create(&conn, json, &title, project.as_deref(), project_prefix.as_deref(), status.as_deref(), priority, dd, ed, description.as_deref())
            }
            TaskAction::List { project, project_prefix, status, priority, pinned, since } => {
                commands::task::list(&conn, json, project.as_deref(), project_prefix.as_deref(), status.as_deref(), priority, pinned, since.as_deref())
            }
            TaskAction::Show { id } => {
                commands::task::show(&conn, json, &id)
            }
            TaskAction::Update { id, title, description, status, priority, due_date, end_date } => {
                commands::task::update(&conn, json, &id, title.as_deref(), description.as_deref(), status.as_deref(), priority, due_date.as_deref(), end_date.as_deref())
            }
            TaskAction::Delete { id } => {
                commands::task::delete(&conn, json, &id)
            }
            TaskAction::Duplicate { id } => {
                commands::task::duplicate(&conn, json, &id)
            }
            TaskAction::Pin { id } => {
                commands::task::toggle_pin(&conn, json, &id, true)
            }
            TaskAction::Unpin { id } => {
                commands::task::toggle_pin(&conn, json, &id, false)
            }
            TaskAction::Move { id, project, project_prefix } => {
                commands::task::move_to_project(&conn, json, &id, project.as_deref(), project_prefix.as_deref())
            }
            TaskAction::BulkDelete { ids } => {
                commands::task::bulk_delete(&conn, json, &parse_ids(&ids))
            }
            TaskAction::BulkStatus { ids, status } => {
                commands::task::bulk_status(&conn, json, &parse_ids(&ids), &status)
            }
            TaskAction::BulkPriority { ids, priority } => {
                commands::task::bulk_priority(&conn, json, &parse_ids(&ids), priority)
            }
            TaskAction::BulkMove { ids, project, project_prefix } => {
                commands::task::bulk_move(&conn, json, &parse_ids(&ids), project.as_deref(), project_prefix.as_deref())
            }
            TaskAction::Trash => {
                commands::task::trash_list(&conn, json)
            }
            TaskAction::Restore { id } => {
                commands::task::restore(&conn, json, &id)
            }
            TaskAction::PermanentDelete { id } => {
                commands::task::permanent_delete(&conn, json, &id)
            }
            TaskAction::EmptyTrash => {
                commands::task::empty_trash(&conn, json)
            }
        },
        Commands::Subtask { action } => match action {
            SubtaskAction::Add { task, title } => {
                commands::subtask::add(&conn, json, &task, &title)
            }
            SubtaskAction::List { task } => {
                commands::subtask::list(&conn, json, &task)
            }
            SubtaskAction::Toggle { id } => {
                commands::subtask::toggle(&conn, json, &id)
            }
            SubtaskAction::Rename { id, title } => {
                commands::subtask::rename(&conn, json, &id, &title)
            }
            SubtaskAction::Delete { id } => {
                commands::subtask::delete(&conn, json, &id)
            }
        },
        Commands::Label { action } => match action {
            LabelAction::Create { name, color } => {
                commands::label::create(&conn, json, &name, &color)
            }
            LabelAction::List => {
                commands::label::list(&conn, json)
            }
            LabelAction::Update { id, name, color } => {
                commands::label::update(&conn, json, &id, name.as_deref(), color.as_deref())
            }
            LabelAction::Delete { id } => {
                commands::label::delete(&conn, json, &id)
            }
            LabelAction::Assign { task, labels } => {
                commands::label::assign(&conn, json, &task, &parse_ids(&labels))
            }
            LabelAction::Show { task } => {
                commands::label::show(&conn, json, &task)
            }
        },
        Commands::Attachment { action } => match action {
            AttachmentAction::Add { task, file } => {
                commands::attachment::add(&conn, json, &task, &file)
            }
            AttachmentAction::List { task } => {
                commands::attachment::list(&conn, json, &task)
            }
            AttachmentAction::Delete { id } => {
                commands::attachment::delete(&conn, json, &id)
            }
            AttachmentAction::Download { id, output } => {
                commands::attachment::download(&conn, json, &id, output.as_deref().unwrap_or(""))
            }
        },
        Commands::Activity { action } => match action {
            ActivityAction::List { task } => {
                commands::activity::list(&conn, json, &task)
            }
        },
        Commands::Data { action } => match action {
            DataAction::Export { output } => {
                commands::data::export(&conn, json, output.as_deref().unwrap_or(""))
            }
            DataAction::Import { file } => {
                commands::data::import(&conn, json, &file)
            }
            DataAction::Backup { keep } => {
                commands::data::backup(json, &db_path, keep.unwrap_or(7))
            }
            DataAction::Restore { name } => {
                commands::data::restore(json, &db_path, &name)
            }
            DataAction::BackupList => {
                commands::data::backup_list(json, &db_path)
            }
            DataAction::BackupDelete { name } => {
                commands::data::backup_delete(json, &db_path, &name)
            }
            DataAction::Reset => {
                commands::data::reset(&conn, json)
            }
        },
        Commands::Settings { action } => match action {
            SettingsAction::Get => {
                commands::settings::get(&conn, json)
            }
            SettingsAction::Set { key, value } => {
                commands::settings::set(&conn, json, &key, &value)
            }
        },
        Commands::Live { action } => match action {
            LiveAction::On { port } => {
                commands::live::on(&conn, json, port)
            }
            LiveAction::Off => {
                commands::live::off(&conn, json)
            }
            LiveAction::Status => {
                commands::live::status(&conn, json)
            }
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
